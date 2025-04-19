use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng}, AeadCore, Aes256Gcm, Key, Nonce
};
use rpassword::read_password;
use std::fs::File;
use std::str;
use std::thread::sleep;
use std::time::Duration;
use argon2::Argon2;  // https://docs.rs/argon2/0.5.3/argon2/ , https://crates.io/crates/argon2
use clap::{builder::Str, Parser, Subcommand};

use image::{DynamicImage, GenericImageView, Pixel, RgbaImage};
use std::io::{self, BufReader, BufWriter, Read, Write};


/// Stegano-Mini
#[derive(Parser, Debug)]
#[command(version, about = "Stegano-Mini")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


#[derive(Subcommand, Debug)]
enum Commands {
    /// Embed data into a PNG image file
    Embed {
        /// Path to the cover PNG image file
        #[arg(short, long, value_name = "COVERFILE")]
        coverfile: String,

        /// Path to the file to embed
        #[arg(short, long, value_name = "EMBEDFILE")]
        embedfile: String,
    },
    /// Extract data from a PNG image file
    Extract {
        /// Path to the stego PNG image file that holds the secret data
        #[arg(short, long, value_name = "STEGOFILE")]
        stegofile: String,
    },
}


fn import_embedfile(file_path: &str) -> Result<Vec<u8>, io::Error> { // io::Result<Vec<u8>> {
    let mut data_file = File::open(file_path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", file_path),
            )
        } else {
            e
        }
    })?;

    let mut data_buffer = Vec::new();
    data_file.read_to_end(&mut data_buffer)?;

    if data_buffer.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "The data buffer is empty.",
        ));
    }

    Ok(data_buffer)
}


fn hide_message_in_image(input_path: &str, output_path: &str, message_bytes: &[u8]) -> Result<(), io::Error> {
    let img = image::open(input_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (width, height) = img.dimensions();
    let mut img = img.to_rgba8();

    let message_bits_count = message_bytes.len() * 8;
    let available_bits = (width * height * 4) as usize; // 4 channels per pixel

    if message_bits_count + 32 > available_bits {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Image is too small to hold the message."));
    }

    let message_len = message_bytes.len() as u32;
    // println!("Message length: {}", message_len);
    // println!("Message Bytes: {:?}", message_bytes);

    let binding = message_len.to_be_bytes();
    let mut message_bits = binding.iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1))
        .chain(message_bytes.iter().flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1)));

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel_mut(x, y);
            let channels = pixel.channels_mut();

            for channel in channels.iter_mut() {
                if let Some(bit) = message_bits.next() {
                    *channel = (*channel & !1) | bit; // !1 == 0b11111110 == 0xFE // pixel[i] = (pixel[i] & 0xFE) | bit; // Modify the LSB of the red channel
                }
            }
        }
    }

    img.save(output_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}


fn extract_message_from_image(image_path: &str) -> Result<Vec<u8>, io::Error> {
    let img = image::open(image_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (width, height) = img.dimensions();
    let img = img.to_rgba8();

    let mut message_bits = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y);
            let channels = pixel.channels();

            for &channel in channels.iter() {
                message_bits.push(channel & 1);
            }
        }
    }

    let message_len_bits = &message_bits[..32];
    let message_len = message_len_bits.iter().enumerate().fold(0u32, |acc, (i, &bit)| {
        acc | ((bit as u32) << (31 - i))
    });

    let message_bits = &message_bits[32..(32 + message_len as usize * 8)];
    let message_bytes: Vec<u8> = message_bits.chunks(8).map(|byte_bits| {
        byte_bits.iter().enumerate().fold(0u8, |acc, (i, &bit)| {
            acc | (bit << (7 - i))
        })
    }).collect();

    Ok(message_bytes)
}


fn hash_password(password: String, salt: &[u8]) -> Result<Vec<u8>, io::Error> {
    let password_bytes: &[u8] = password.as_bytes();
    // The Aes256Gcm cipher requires a 256-bit key (32 bytes)
    let mut hashed_password: Vec<u8> = vec![0u8; 32];
    // Argon2 with default params (Argon2id v19)
    Argon2::default()
        .hash_password_into(password_bytes, salt, &mut hashed_password)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(hashed_password)
}


fn generate_random_key(length: usize) -> Vec<u8> {
    let mut rng: OsRng = OsRng;
    let mut key: Vec<u8> = vec![0u8; length];
    rng.fill_bytes(&mut key);
    key
}


fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let nonce_size: usize = 12; // In AES-GCM, the nonce size is typically 96 bits (12 bytes)
    let salt_size: usize = 16;  // Used by Argon2

    match &cli.command {
        Commands::Embed { coverfile, embedfile } => {
            let outputfile: &str = "output.png";
            println!("Embedding file: {} into cover file: {}", embedfile, coverfile);
            println!("Write result back to file: {}", outputfile);

            if !coverfile.to_lowercase().ends_with(".png") {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,"The cover image must be of PNG format"));
            }

            if !embedfile.to_lowercase().ends_with(".txt") {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,"The embed file must be of TXT format"));
            }
            // let path = std::path::Path::new(embedfile);
            // if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
            //     return Err(io::Error::new(io::ErrorKind::InvalidInput,"The embed file must be of TXT format"));
            // }
        
            let plaintext: Vec<u8> = import_embedfile(embedfile)?;
            
            // Generate Hash from Password
            let password: String = get_user_input(true)?;
            let salt: Vec<u8> = generate_random_key(salt_size); // Salt should be unique per password
            let hashed_password: Vec<u8> = match hash_password(password, &salt) {
                Ok(hash) => {
                    // println!("Password hashed successfully.");
                    hash
                }
                Err(e) => {
                    eprintln!("Error hashing password: {}", e);
                    return Err(e);
                }
            };

            // Generate Key from Hash
            let key = Key::<Aes256Gcm>::from_slice(&hashed_password);
            let cipher = Aes256Gcm::new(key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // nonce (or initialization vector, IV); 96-bits (12 bytes); unique per message
            // {
            //     let key = Aes256Gcm::generate_key(&mut OsRng);
            //     let cipher = Aes256Gcm::new(&key);
            //     let nonce = Nonce::from_slice(b"unique nonce");  // Example nonce (12 bytes)
            // }
            assert_eq!(nonce.len(), nonce_size);


            // Encrypt:
            let ciphertext: Vec<u8> = cipher
                .encrypt(&nonce, plaintext.as_ref())
                .expect("encryption failure!");
            assert_ne!(&ciphertext, &plaintext);

            // let combined: Vec<u8> = [nonce.to_vec(), salt, ciphertext.clone()].concat();
            let mut combined: Vec<u8> = nonce.to_vec();
            combined.extend_from_slice(&salt);
            combined.extend_from_slice(&ciphertext);

            // Write data to image
            hide_message_in_image(coverfile, outputfile, combined.as_slice())?; // data_vec.as_slice() == &data_vec

            // Introduce a 1-second delay
            // sleep(Duration::from_secs(1));
            sleep(Duration::from_millis(1_000));

            // Test recovering hidden message
            let extracted_message: Vec<u8> = extract_message_from_image(outputfile)?;
            if extracted_message == combined {
                println!("Job finished!");
            } else {
                println!("Test failed: The extracted message does not match the original!");
            }
        }
        Commands::Extract { stegofile } => {
            println!("Extracting from stego file: {}", stegofile);
            
            let recovered_data: Vec<u8> = extract_message_from_image(stegofile)?;

            // Decrypt:
            let (nonce_slice, salt_ciphertext_bytes): (&[u8], &[u8]) = recovered_data.split_at(nonce_size);
            let (salt_bytes, ciphertext_bytes): (&[u8], &[u8]) = salt_ciphertext_bytes.split_at(salt_size);
            let nonce_bytes: &[u8; 12] = match nonce_slice.try_into() {
                Ok(array_ref) => array_ref,
                Err(_) => panic!("Slice length mismatch!"),
            };
            let extracted_nonce = Nonce::from_slice(nonce_bytes);

            // Generate Hash from Password
            let password: String = get_user_input(false)?;
            let hashed_password: Vec<u8> = match hash_password(password, &salt_bytes) {
                Ok(hash) => {
                    // println!("Password hashed successfully.");
                    hash
                }
                Err(e) => {
                    eprintln!("Error hashing password: {}", e);
                    return Err(e);
                }
            };

            // Generate Key from Hash
            let key = Key::<Aes256Gcm>::from_slice(&hashed_password);
            let cipher = Aes256Gcm::new(key);

            // Decrypt ciphertext_bytes
            let decryption_result: Result<Vec<u8>, _> = cipher.decrypt(&extracted_nonce, ciphertext_bytes.as_ref());

            match decryption_result {
                Ok(decrypted_plaintext) => {
                    // Write the plaintext to a text file
                    let mut file = File::create("output.txt").map_err(|e| {
                        eprintln!("Failed to create file: {}", e);
                        e
                    })?;
                    file.write_all(&decrypted_plaintext).map_err(|e| {
                        eprintln!("Failed to write to file: {}", e);
                        e
                    })?;
                }
                Err(_) => {
                    eprintln!("Decryption failed: Incorrect password or corrupted data.");
                    return Err(io::Error::new(io::ErrorKind::Other, "Decryption failed"));
                }
            }
            
            println!("Job finished!");
        }
    }

    Ok(())
}


fn get_user_input(confirm: bool) -> Result<String, std::io::Error> {
    loop {
        print!("Please enter a passphrase: ");
        std::io::stdout().flush()?;
        let user_input: String = read_password()?;

        if confirm {
            if user_input.len() < 12 {
                println!("Input too short. Your passphrase must be at least 12 characters long. Please ensure it includes a mix of letters, numbers, and special characters to enhance security.\n");
                continue;
            }
    
            print!("Please re-enter your passphrase for confirmation: ");
            std::io::stdout().flush()?;
            let confirm_input: String = read_password()?;

            if user_input == confirm_input {
                return Ok(user_input);
            } else {
                println!("Passphrases do not match. Please try again.\n");
            }
        } else {
            return Ok(user_input);
        }
    }
}
