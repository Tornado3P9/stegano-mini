// #[macro_use]
// extern crate fstrings;

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng}, AeadCore, Aes256Gcm, Key, Nonce
};
use image::GenericImageView;
use rpassword::read_password;
use std::fs::File;
use std::io::{self, Read, Write};
use std::str;
use std::thread::sleep;
use std::time::Duration;
use argon2::Argon2;  // https://docs.rs/argon2/0.5.3/argon2/ , https://crates.io/crates/argon2
use clap::{Parser, Subcommand};


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


fn import_secret_text_file(file_path: &str) -> io::Result<Vec<u8>> {
    // // Check if the file has a .txt extension
    // let path = std::path::Path::new(file_path);
    // if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
    //     return Err(io::Error::new(
    //         io::ErrorKind::InvalidInput,
    //         "The file is not a .txt file.",
    //     ));
    // }
    if !file_path.to_lowercase().ends_with(".txt") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The file is not a .txt file.",
        ));
    }

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


fn steghide_png_image(image_path: &str, secret: &[u8]) -> io::Result<()> {
    // Check if the file is a PNG
    if !image_path.to_lowercase().ends_with(".png") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The image must be a PNG file",
        ));
    }

    // Load the image file
    let img = image::open(image_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (width, height) = img.dimensions();
    let mut img_buffer = img.to_rgb8();

    // Check if the image can hold the secret and the 4 pixels for length
    if (width * height) < (secret.len() as u32 + 4) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Image dimensions are insufficient to hold the secret data and length encoding",
        ));
    }

    // Encode the length of the secret in the first 4 pixels
    let secret_length = secret.len() as u32;
    for i in 0..4 {
        let x = i as u32;
        let y = 0;
        let pixel = img_buffer.get_pixel_mut(x, y);
        pixel[0] = (secret_length >> (8 * i)) as u8;
    }

    // Embed the secret starting after the length encoding
    for (i, &byte) in secret.iter().enumerate() {
        let index = i + 4; // Start after the first 4 pixels
        let x = (index as u32) % width;
        let y = (index as u32) / width;
        if y >= height {
            break;
        }
        let pixel = img_buffer.get_pixel_mut(x, y);
        pixel[0] = byte; // Consider using pixel[1] and pixel[2] for more capacity, currently using red component for both length encoding and secret embedding.
    }

    img_buffer
        .save("output.png")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}


fn recover_secret_from_image(image_path: &str) -> io::Result<Vec<u8>> {
    // Check if the file is a PNG
    if !image_path.to_lowercase().ends_with(".png") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The image must be a PNG file",
        ));
    }

    // Load the image file
    let img = image::open(image_path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let (width, height) = img.dimensions();
    let img_buffer = img.to_rgb8();

    // Decode the length of the secret from the first 4 pixels
    let mut secret_length = 0u32;
    for i in 0..4 {
        let x = i as u32;
        let y = 0;
        let pixel = img_buffer.get_pixel(x, y);
        secret_length |= (pixel[0] as u32) << (8 * i);
    }

    // Use the decoded length to extract the secret
    let mut secret = Vec::new();
    for i in 0..secret_length as usize {
        let index = i + 4; // Start after the first 4 pixels
        let x = (index as u32) % width;
        let y = (index as u32) / width;
        if y >= height {
            break;
        }
        let pixel = img_buffer.get_pixel(x, y);
        secret.push(pixel[0]); // Remember to change this line too if you decide to also use pixel[1] and pixel[2] in steghide_png_image()
    }

    Ok(secret)
}


fn hash_password(password: &[u8], salt: &[u8]) -> Result<Vec<u8>, io::Error> {
    // The Aes256Gcm cipher requires a 256-bit key (32 bytes)
    let mut hashed_password: Vec<u8> = vec![0u8; 32];
    // Argon2 with default params (Argon2id v19)
    Argon2::default()
        .hash_password_into(password, salt, &mut hashed_password)
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
            println!("Embedding file: {} into cover file: {}", embedfile, coverfile);

            let plaintext: Vec<u8> = import_secret_text_file(embedfile)?;
            
            // Generate Hash from Password
            let password: String = get_user_input(true)?;
            let password_bytes: &[u8] = password.as_bytes();
            let salt: Vec<u8> = generate_random_key(salt_size); // Salt should be unique per password
            let hashed_password: Vec<u8> = match hash_password(password_bytes, &salt) {
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
            // let key = Aes256Gcm::generate_key(&mut OsRng);
            // let cipher = Aes256Gcm::new(&key);
            // let nonce = Nonce::from_slice(b"unique nonce");  // Example nonce (12 bytes)
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
            steghide_png_image(coverfile, &combined)?;
            // println!("ciphertext.len(): {:?}", ciphertext.len());
            // println!("combined.len(): {:?}", combined.len());

            // Introduce a 1-second delay
            sleep(Duration::from_secs(1));

            // Recover the secret from the image
            let recovered_data: Vec<u8> = recover_secret_from_image("output.png")?;

            // assert_eq!(recovered_data, combined);
            if recovered_data == combined {
                println!("Job finished!");
            } else {
                println!("combined data: {:?} ...first ten items", &combined[0..10]);
                println!("recovered_data: {:?} ...first ten items", &recovered_data[0..10]);
                println!("Test failed: recovered_data from the embedded image does not match the original!");
            }
        }
        Commands::Extract { stegofile } => {
            println!("Extracting from stego file: {}", stegofile);
            
            // Recover the secret from the image
            let recovered_data: Vec<u8> = recover_secret_from_image(stegofile)?;
            // println!("recovered_data: {:?} ...first ten items", &recovered_data[0..10]);

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
            let password_bytes: &[u8] = password.as_bytes();
            let hashed_password: Vec<u8> = match hash_password(password_bytes, &salt_bytes) {
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
