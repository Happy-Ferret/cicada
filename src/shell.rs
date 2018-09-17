use errno::errno;
use libc;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::mem;

use glob;
use regex::Regex;

use execute;
use parsers;
use tools::{self, clog};

#[derive(Debug, Clone)]
pub struct Shell {
    pub alias: HashMap<String, String>,
    pub envs: HashMap<String, String>,
    pub cmd: String,
    pub previous_dir: String,
    pub previous_cmd: String,
    pub previous_status: i32,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            alias: HashMap::new(),
            envs: HashMap::new(),
            cmd: String::new(),
            previous_dir: String::new(),
            previous_cmd: String::new(),
            previous_status: 0,
        }
    }

    pub fn set_env(&mut self, name: &str, value: &str) {
        if let Ok(_) = env::var(name) {
            env::set_var(name, value);
        } else {
            self.envs.insert(name.to_string(), value.to_string());
        }
    }

    pub fn get_env(&self, name: &str) -> Option<String> {
        match self.envs.get(name) {
            Some(x) => {
                Some(x.to_string())
            }
            None => {
                None
            }
        }
    }

    pub fn add_alias(&mut self, name: &str, value: &str) {
        self.alias.insert(name.to_string(), value.to_string());
    }

    pub fn get_alias_content(&self, name: &str) -> Option<String> {
        let mut result;
        match self.alias.get(name) {
            Some(x) => {
                result = x.to_string();
            }
            None => {
                result = String::new();
            }
        }
        tools::pre_handle_cmd_line(self, &mut result);
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

pub unsafe fn give_terminal_to(gid: i32) -> bool {
    let mut mask: libc::sigset_t = mem::zeroed();
    let mut old_mask: libc::sigset_t = mem::zeroed();

    libc::sigemptyset(&mut mask);
    libc::sigaddset(&mut mask, libc::SIGTSTP);
    libc::sigaddset(&mut mask, libc::SIGTTIN);
    libc::sigaddset(&mut mask, libc::SIGTTOU);
    libc::sigaddset(&mut mask, libc::SIGCHLD);

    let rcode = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut old_mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    let rcode = libc::tcsetpgrp(1, gid);
    let given;
    if rcode == -1 {
        given = false;
        let e = errno();
        let code = e.0;
        log!("Error {}: {}", code, e);
    } else {
        given = true;
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    given
}

fn needs_globbing(line: &str) -> bool {
    if tools::is_arithmetic(line) {
        return false;
    }

    let re;
    if let Ok(x) = Regex::new(r"\*+") {
        re = x;
    } else {
        return false;
    }

    let tokens = parsers::parser_line::cmd_to_tokens(line);
    for (sep, token) in tokens {
        if !sep.is_empty() {
            continue;
        }
        if re.is_match(&token) {
            return true;
        }
    }
    false
}

pub fn expand_glob(tokens: &mut Vec<(String, String)>) {
    let mut idx: usize = 0;

    let mut buff: HashMap<usize, String> = HashMap::new();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !needs_globbing(text) {
            idx += 1;
            continue;
        }

        let _line = text.to_string();
        // XXX: spliting needs to consider cases like `echo 'a * b'`
        let _tokens: Vec<&str> = _line.split(' ').collect();
        let mut result: Vec<String> = Vec::new();
        for item in &_tokens {
            if !item.contains('*') || item.trim().starts_with('\'') || item.trim().starts_with('"') {
                result.push(item.to_string());
            } else {
                match glob::glob(item) {
                    Ok(paths) => {
                        let mut is_empty = true;
                        for entry in paths {
                            match entry {
                                Ok(path) => {
                                    let s = path.to_string_lossy();
                                    if !item.starts_with('.') && s.starts_with('.') && !s.contains('/')
                                    {
                                        // skip hidden files, you may need to
                                        // type `ls .*rc` instead of `ls *rc`
                                        continue;
                                    }
                                    result.push(tools::wrap_sep_string("", &s));
                                    is_empty = false;
                                }
                                Err(e) => {
                                    log!("glob error: {:?}", e);
                                }
                            }
                        }
                        if is_empty {
                            result.push(item.to_string());
                        }
                    }
                    Err(e) => {
                        println!("glob error: {:?}", e);
                        result.push(item.to_string());
                        return;
                    }
                }
            }
        }
        // *line = result.join(" ");

        buff.insert(idx, result.join(" "));
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

pub fn extend_glob(line: &mut String) {
    if !needs_globbing(&line) {
        return;
    }
    let _line = line.clone();
    // XXX: spliting needs to consider cases like `echo 'a * b'`
    let _tokens: Vec<&str> = _line.split(' ').collect();
    let mut result: Vec<String> = Vec::new();
    for item in &_tokens {
        if !item.contains('*') || item.trim().starts_with('\'') || item.trim().starts_with('"') {
            result.push(item.to_string());
        } else {
            match glob::glob(item) {
                Ok(paths) => {
                    let mut is_empty = true;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                let s = path.to_string_lossy();
                                if !item.starts_with('.') && s.starts_with('.') && !s.contains('/')
                                {
                                    // skip hidden files, you may need to
                                    // type `ls .*rc` instead of `ls *rc`
                                    continue;
                                }
                                result.push(tools::wrap_sep_string("", &s));
                                is_empty = false;
                            }
                            Err(e) => {
                                log!("glob error: {:?}", e);
                            }
                        }
                    }
                    if is_empty {
                        result.push(item.to_string());
                    }
                }
                Err(e) => {
                    println!("glob error: {:?}", e);
                    result.push(item.to_string());
                    return;
                }
            }
        }
    }
    *line = result.join(" ");
}

pub fn extend_env_blindly(sh: &Shell, token: &str) -> String {
    let re;
    if let Ok(x) = Regex::new(r"([^\$]*)\$\{?([A-Za-z0-9\?\$_]+)\}?(.*)") {
        re = x;
    } else {
        println!("cicada: re new error");
        return String::new();
    }
    if !re.is_match(token) {
        return token.to_string();
    }
    let mut result = String::new();
    let mut _token = token.to_string();
    let mut _head = String::new();
    let mut _output = String::new();
    let mut _tail = String::new();
    loop {
        if !re.is_match(&_token) {
            if !_token.is_empty() {
                result.push_str(&_token);
            }
            break;
        }
        for cap in re.captures_iter(&_token) {
            _head = cap[1].to_string();
            _tail = cap[3].to_string();
            let _key = cap[2].to_string();
            if _key == "?" {
                result.push_str(format!("{}{}", _head, sh.previous_status).as_str());
            } else if _key == "$" {
                unsafe {
                    let val = libc::getpid();
                    result.push_str(format!("{}{}", _head, val).as_str());
                }
            } else if let Ok(val) = env::var(&_key) {
                result.push_str(format!("{}{}", _head, val).as_str());
            } else {
                if let Some(val) = sh.get_env(&_key) {
                    result.push_str(format!("{}{}", _head, val).as_str());
                } else {
                    result.push_str(&_head);
                }
            }
        }
        if _tail.is_empty() {
            break;
        }
        _token = _tail.clone();
    }
    result
}

pub fn extend_env(sh: &Shell, line: &mut String) {
    let mut result: Vec<String> = Vec::new();
    let _line = line.clone();
    let args = parsers::parser_line::cmd_to_tokens(_line.as_str());
    for (sep, token) in args {
        if sep == "`" || sep == "'" {
            result.push(tools::wrap_sep_string(&sep, &token));
        } else {
            let _token = extend_env_blindly(sh, &token);
            result.push(tools::wrap_sep_string(&sep, &_token));
        }
    }
    *line = result.join(" ");
}

fn expand_brace(tokens: &mut Vec<(String, String)>) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, String> = HashMap::new();
    for (sep, line) in tokens.iter() {
        if !sep.is_empty() || !tools::should_extend_brace(&line) {
            idx += 1;
            continue;
        }

        let _line = line.clone();
        let args = parsers::parser_line::cmd_to_tokens(_line.as_str());
        let mut result: Vec<String> = Vec::new();
        for (sep, token) in args {
            if sep.is_empty() && tools::should_extend_brace(token.as_str()) {
                let mut _prefix = String::new();
                let mut _token = String::new();
                let mut _result = Vec::new();
                let mut only_tail_left = false;
                let mut start_sign_found = false;
                for c in token.chars() {
                    if c == '{' {
                        start_sign_found = true;
                        continue;
                    }
                    if !start_sign_found {
                        _prefix.push(c);
                        continue;
                    }
                    if only_tail_left {
                        _token.push(c);
                        continue;
                    }
                    if c == '}' {
                        if !_token.is_empty() {
                            _result.push(_token);
                            _token = String::new();
                        }
                        only_tail_left = true;
                        continue;
                    }
                    if c == ',' {
                        if !_token.is_empty() {
                            _result.push(_token);
                            _token = String::new();
                        }
                    } else {
                        _token.push(c);
                    }
                }
                for item in &mut _result {
                    *item = format!("{}{}{}", _prefix, item, _token);
                }
                result.push(_result.join(" "));
            } else {
                result.push(tools::wrap_sep_string(&sep, &token));
            }
        }

        buff.insert(idx, result.join(" "));
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

pub fn expand_home_string(text: &mut String) {
    // let mut s: String = String::from(text);
    let v = vec![
        r"(?P<head> +)~(?P<tail> +)",
        r"(?P<head> +)~(?P<tail>/)",
        r"^(?P<head> *)~(?P<tail>/)",
        r"(?P<head> +)~(?P<tail> *$)",
    ];
    for item in &v {
        let re;
        if let Ok(x) = Regex::new(item) {
            re = x;
        } else {
            return;
        }
        let home = tools::get_user_home();
        let ss = text.clone();
        let to = format!("$head{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        *text = result.to_string();
    }
}

fn expand_home(tokens: &mut Vec<(String, String)>) {
    let mut idx: usize = 0;

    let mut buff: HashMap<usize, String> = HashMap::new();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !needs_expand_home(&text) {
            idx += 1;
            continue;
        }

        let mut s: String = text.clone();
        let v = vec![
            r"(?P<head> +)~(?P<tail> +)",
            r"(?P<head> +)~(?P<tail>/)",
            r"^(?P<head> *)~(?P<tail>/)",
            r"(?P<head> +)~(?P<tail> *$)",
        ];
        for item in &v {
            let re;
            if let Ok(x) = Regex::new(item) {
                re = x;
            } else {
                return;
            }
            let home = tools::get_user_home();
            let ss = s.clone();
            let to = format!("$head{}$tail", home);
            let result = re.replace_all(ss.as_str(), to.as_str());
            s = result.to_string();
        }
        buff.insert(idx, s.clone());
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn expand_env(sh: &Shell, tokens: &mut Vec<(String, String)>) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, String> = HashMap::new();

    for (sep, line) in tokens.iter() {
        if sep == "`" || sep == "'" {
            idx += 1;
            continue;
        }

        let _token = extend_env_blindly(sh, line);
        buff.insert(idx, _token);
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn should_do_dollar_command_extension(line: &str) -> bool {
    tools::re_contains(line, r"\$\([^\)]+\)")
}

fn should_do_dot_command_extension(line: &str) -> bool {
    tools::re_contains(line, r"`[^`]+`")
}


fn do_command_substitution(tokens: &mut Vec<(String, String)>) {
    do_command_substitution_for_dot(tokens);
    do_command_substitution_for_dollar(tokens);
}

pub fn do_expansion(sh: &Shell, tokens: &mut Vec<(String, String)>) {
    expand_home(tokens);
    expand_brace(tokens);
    expand_env(sh, tokens);
    expand_glob(tokens);
    do_command_substitution(tokens);
}

pub fn needs_expand_home(line: &str) -> bool {
    tools::re_contains(line, r"( +~ +)|( +~/)|(^ *~/)|( +~ *$)")
}

#[cfg(test)]
mod tests {
    use stl::fs::{self, File};

    use super::extend_env;
    use super::extend_glob;
    use super::needs_expand_home;
    use super::needs_globbing;
    use super::Shell;
    use super::should_do_dollar_command_extension;

    #[test]
    fn test_need_expand_home() {
        assert!(needs_expand_home("ls ~"));
        assert!(needs_expand_home("ls  ~  "));
        assert!(needs_expand_home("cat ~/a.py"));
        assert!(needs_expand_home("echo ~"));
        assert!(needs_expand_home("echo ~ ~~"));
        assert!(needs_expand_home("~/bin/py"));
        assert!(!needs_expand_home("echo '~'"));
        assert!(!needs_expand_home("echo \"~\""));
        assert!(!needs_expand_home("echo ~~"));
    }

    #[test]
    fn test_needs_globbing() {
        assert!(needs_globbing("*"));
        assert!(needs_globbing("ls *"));
        assert!(needs_globbing("ls  *.txt"));
        assert!(needs_globbing("grep -i 'desc' /etc/*release*"));
        assert!(!needs_globbing("2 * 3"));
        assert!(!needs_globbing("ls '*.md'"));
        assert!(!needs_globbing("ls 'a * b'"));
        assert!(!needs_globbing("ls foo"));
    }

    #[test]
    fn test_extend_env() {
        let sh = Shell::new();
        let mut s = String::from("echo '$PATH'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo '$PATH'");

        let mut s = String::from("echo a\\ b xy");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo a\\ b xy");

        let mut s = String::from("echo 'hi $PATH'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo 'hi $PATH'");

        let mut s = String::from("echo \'\\\'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo \'\\\'");

        let mut s = String::from("export DIR=`brew --prefix openssl`/include");
        extend_env(&sh, &mut s);
        assert_eq!(s, "export DIR=`brew --prefix openssl`/include");

        let mut s = String::from("export FOO=\"`date` and `go version`\"");
        extend_env(&sh, &mut s);
        assert_eq!(s, "export FOO=\"`date` and `go version`\"");

        let mut s = String::from("foo is XX${CICADA_NOT_EXIST}XX");
        extend_env(&sh, &mut s);
        assert_eq!(s, "foo is XXXX");

        let mut s = String::from("foo is $CICADA_NOT_EXIST_1 and bar is $CICADA_NOT_EXIST_2.");
        extend_env(&sh, &mut s);
        assert_eq!(s, "foo is  and bar is .");
    }

    #[test]
    fn test_extend_glob() {
        let fname = "foo bar baz.txt";
        File::create(fname).expect("error when create file");
        let mut line = String::from("echo f*z.txt");
        extend_glob(&mut line);
        fs::remove_file(fname).expect("error when rm file");
        assert_eq!(line, "echo foo\\ bar\\ baz.txt");

        line = String::from("echo bar*.txt");
        extend_glob(&mut line);
        assert_eq!(line, "echo bar*.txt");

        line = String::from("echo \"*\"");
        extend_glob(&mut line);
        assert_eq!(line, "echo \"*\"");

        line = String::from("echo \'*\'");
        extend_glob(&mut line);
        assert_eq!(line, "echo \'*\'");
    }

    #[test]
    fn test_should_do_dollar_command_extension() {
        assert!(!should_do_dollar_command_extension("ls $HOME"));
        assert!(!should_do_dollar_command_extension("echo $[pwd]"));
        assert!(should_do_dollar_command_extension("echo $(pwd)"));
        assert!(should_do_dollar_command_extension("echo $(pwd) foo"));
        assert!(should_do_dollar_command_extension("echo $(foo bar)"));
        assert!(should_do_dollar_command_extension("echo $(echo foo)"));
        assert!(should_do_dollar_command_extension("$(pwd) foo"));
    }
}
