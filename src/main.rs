extern crate rustc_serialize;
extern crate docopt;
extern crate regex;

use docopt::Docopt;
use regex::Regex;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

// Write the Docopt usage string.
static USAGE: &'static str = "
Usage: istringscheck <source> <translations>
Options:
    -h, --help  Displays this message.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_source: String,
    arg_translations: String,
}


// Ensures that all your translated strings have the same number of format parameters
// as your source strings.
fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());
    validate_args(&args);

    // Input Strings
    let source_filename = &args.arg_source;
    let source_strings = hashmap_from_source(path_from_string(source_filename), "utf-8");

    // Strings to compare to
    let mut language_files = language_files_from_dir(&args.arg_translations);

    // Don't compare the source strings with itself.
    language_files.iter().position(|file| file.to_str().eq(&path_from_string(source_filename).to_str()) ).map(|e| language_files.remove(e));

    for language_file in language_files {
        println!( "Parsing language file {}", language_file.display());

        let translated_strings = hashmap_from_source(language_file, "utf-8");

        compare_strings(&source_strings, &translated_strings);

    }
}

fn compare_strings(source_strings :&HashMap<String, usize>, translated_strings: &HashMap<String, usize>) {
    for (key, value) in source_strings {
        let translated_value = translated_strings.get(key).unwrap_or_else(|| panic!("Language file is missing strings for key {}", key));

        if translated_value != value {
            println!("This attributed string: {} doesn't have the correct amount of occurences of the format argument", key);
        }
    }
}

fn language_files_from_dir(dir_string: &String) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(&Path::new(dir_string)).unwrap();
    let mut string_files :Vec<PathBuf> = vec![];

    loop {
        match paths.next() {
            Some(x) => {
                let path = x.unwrap().path();
                let path_string: String = path.to_str().unwrap().to_string();

                if is_folder(path.clone()){
                    for file in &language_files_from_dir(&path_string) {
                        string_files.push(file.clone());
                    }
                } else {
                    let extension = path.extension().unwrap_or(&OsString::new()).to_os_string().into_string().unwrap();
                    if extension == "strings" {
                        string_files.push(path);
                    }
                }
            },
            None => { break }
        }
    }

    return string_files;
}

fn hashmap_from_source(source_path: PathBuf, file_encoding: &str) -> HashMap<String, usize> {
    let mut source_strings: HashMap<String, usize> = HashMap::new();
    // Apple Strings files are UTF-16 encoded, processing them in UTF-8
    let utf8_conversion_out = Command::new("iconv")
                                .arg("-f").arg(file_encoding)
                                .arg("-t").arg("utf-8")
                                .arg(&source_path).output()
                                .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });

    let strings_file = String::from_utf8(utf8_conversion_out.stdout).unwrap();
    let mut lines  = strings_file.lines();

    let string_key_re = match Regex::new(r#"^"(.*)" = "(.*)";$"#) {
        Ok(re) => re,
        Err(err) => panic!("{}", err),
    };
    let format_strings_re = match Regex::new(r#"%"#) {
        Ok(re) => re,
        Err(err) => panic!("{}", err),
    };

    loop {
        match lines.next() {
            Some(line) => {
                for cap in string_key_re.captures_iter(&line) {
                    let n_formats = format_strings_re.captures_iter(&cap.get(2).unwrap().as_str()).count();
                    source_strings.insert(cap.get(1).unwrap().as_str().to_string(), n_formats);
                }
            },
            None => { break }
        }
    }

    return source_strings;
}

fn path_from_string(path_string: &String) -> PathBuf {
    return Path::new(path_string).to_path_buf();
}

fn is_file(path :PathBuf) -> bool {
    return fs::metadata(path).map(|m| m.is_file()).unwrap_or(false);
}

fn is_folder(path :PathBuf) -> bool {
    return fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false);
}

// Validates the arguments, checking file and folder exists.
fn validate_args(args: &Args) {
    let source_path = path_from_string(&args.arg_source);
    let dir_path    = path_from_string(&args.arg_translations);

    let source_exists = is_file(source_path);
    let dir_exists    = is_folder(dir_path);

    if !(source_exists && dir_exists) {
        panic!("The arguments passed must be the source localization files
        and the folder containing other localizations \n".to_string() + USAGE);
    }
}
