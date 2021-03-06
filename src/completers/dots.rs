use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use linefeed::complete::escape;
use linefeed::complete::escaped_word_start;
use linefeed::complete::unescape;
use linefeed::complete::Suffix;
use linefeed::complete::{Completer, Completion};
use linefeed::prompter::Prompter;
use linefeed::terminal::Terminal;

use yaml_rust::yaml;
use yaml_rust::YamlLoader;

use parsers;
use tools;

/// Performs completion by searching dotfiles
pub struct DotsCompleter;

impl<Term: Terminal> Completer<Term> for DotsCompleter {
    fn complete(
        &self,
        word: &str,
        reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let line = reader.buffer();
        Some(complete_dots(line, word))
    }

    fn word_start(&self, line: &str, end: usize, _reader: &Prompter<Term>) -> usize {
        escaped_word_start(&line[..end])
    }

    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        escape(word)
    }

    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        unescape(word)
    }
}

fn complete_dots(line: &str, word: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let args = parsers::parser_line::line_to_plain_tokens(line);
    if args.is_empty() {
        return res;
    }
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    let dot_file = dot_file.as_str();
    if !Path::new(dot_file).exists() {
        return res;
    }
    let sub_cmd = if (args.len() >= 3 && !args[1].starts_with('-'))
        || (args.len() >= 2 && !args[1].starts_with('-') && line.ends_with(' '))
    {
        args[1].as_str()
    } else {
        ""
    };

    let mut f;
    match File::open(dot_file) {
        Ok(x) => f = x,
        Err(e) => {
            println!("cicada: open dot_file error: {:?}", e);
            return res;
        }
    }
    let mut s = String::new();
    match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(e) => {
            println!("cicada: read_to_string error: {:?}", e);
            return res;
        }
    }

    let docs;
    match YamlLoader::load_from_str(&s) {
        Ok(x) => {
            docs = x;
        }
        Err(_) => {
            println_stderr!("\ncicada: Bad Yaml file: {}?", dot_file);
            return res;
        }
    }
    for doc in &docs {
        match *doc {
            yaml::Yaml::Array(ref v) => {
                for x in v {
                    match *x {
                        yaml::Yaml::String(ref name) => {
                            if sub_cmd != "" || !name.starts_with(word) {
                                continue;
                            }

                            let display = None;
                            let suffix = Suffix::Default;
                            res.push(Completion {
                                completion: name.to_string(),
                                display,
                                suffix,
                            });
                        }
                        yaml::Yaml::Hash(ref h) => {
                            for (k, v) in h.iter() {
                                if let yaml::Yaml::String(ref name) = *k {
                                    if sub_cmd != "" && sub_cmd != name {
                                        continue;
                                    }
                                    if sub_cmd == "" {
                                        if !name.starts_with(word) {
                                            continue;
                                        }

                                        let name = name.clone();
                                        let display = None;
                                        let suffix = Suffix::Default;
                                        res.push(Completion {
                                            completion: name,
                                            display,
                                            suffix,
                                        });
                                    } else if let yaml::Yaml::Array(ref v) = *v {
                                        for x in v {
                                            if let yaml::Yaml::String(ref name) = *x {
                                                if !name.starts_with(word) {
                                                    continue;
                                                }

                                                let name = name.clone();
                                                let display = None;
                                                let suffix = Suffix::Default;
                                                res.push(Completion {
                                                    completion: name,
                                                    display,
                                                    suffix,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                println!("Found unknown yaml doc");
            }
        }
    }
    res
}
