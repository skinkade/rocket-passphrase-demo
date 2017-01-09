#![feature(custom_derive)]
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rand;
extern crate rocket;
extern crate rocket_contrib;

#[macro_use] extern crate lazy_static;

use std::sync::Mutex;
use rand::{Rng, SeedableRng};
use rand::os::OsRng;
use rand::chacha::ChaChaRng;
use rocket_contrib::Template;


lazy_static! {
    /*
     * While a global ChaCha20 CSPRNG does reduce need of e.g. /dev/urandom
     * to only at initial runtime, I mostly did this just to see if I could :)
     */
    static ref CHACHA_MUTEX: Mutex<ChaChaRng> = {
        let mut os_csprng = match OsRng::new() {
            Ok(g) => g,
            Err(e) => panic!("{}", e),
        };

        // 256-bit seed
        let mut seed = [0u32; 8];
        for i in 0..8 {
            seed[i] = os_csprng.next_u32();
        }

        Mutex::new(ChaChaRng::from_seed(&seed))
    };

    // Bakes our wordlist straight in
    static ref WORDLIST: Vec<&'static str> = include_str!("../corncob_lowercase.txt")
                                                .lines()
                                                .collect();

    static ref WORDLIST_LEN: usize = WORDLIST.len();
}


// https://stackoverflow.com/a/38406885
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}


fn passphrase_gen(num_words: u8, num_digits: u8) -> String {
    let mut csprng = CHACHA_MUTEX.lock().unwrap();
    let mut passphrase: String;

    // Always at least one word
    // Don't both allocating vec for just one string
    if num_words <= 1 {
        let word = WORDLIST[csprng.gen_range(0, *WORDLIST_LEN)];
        passphrase = capitalize(word);
    } else {
        let mut words = Vec::with_capacity(num_words as usize);
        for _ in 0..num_words {
            let word = WORDLIST[csprng.gen_range(0, *WORDLIST_LEN)];
            words.push(capitalize(word));
        }
        passphrase = words.join("-");
    }

    if num_digits > 0 {
        for _ in 0..num_digits {
            let digit = csprng.gen_range(0, 10);
            passphrase.push_str(&digit.to_string());
        }
    }

    passphrase
}


#[derive(FromForm)]
struct PhraseParams {
    length: Option<u8>,
    digits: Option<u8>
}

static DEFAULT_LENGTH: u8 = 4;
static DEFAULT_DIGITS: u8 = 0;


#[get("/passphrase?<params>")]
fn phrase_params(params: PhraseParams) -> String {
    let num_words = params.length.unwrap_or(DEFAULT_LENGTH);
    let num_digits = params.digits.unwrap_or(DEFAULT_DIGITS);

    passphrase_gen(num_words, num_digits)
}

#[get("/passphrase", rank = 2)]
fn phrase() -> String {
    passphrase_gen(DEFAULT_LENGTH, DEFAULT_DIGITS)
}

# [get("/")]
fn index() -> Template {
    let mut context = std::collections::HashMap::new();
    context.insert("num_words", DEFAULT_LENGTH);
    context.insert("num_digits", DEFAULT_DIGITS);
    Template::render("index", &context)
}


fn main() {
    rocket::ignite()
        .mount("/", routes![index, phrase_params, phrase])
        .launch();
}
