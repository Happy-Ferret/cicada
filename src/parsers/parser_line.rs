pub fn parse_line(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let v = parse_args(line);
    for (_, r) in v {
        result.push(r);
    }
    return result;
}

pub fn parse_commands(line: &str) -> Vec<String> {
    // Special characters: http://tldp.org/LDP/abs/html/special-chars.html
    let mut result = Vec::new();
    let mut sep = String::new();
    let mut token = String::new();
    let len = line.len();
    for (i, c) in line.chars().enumerate() {
        if c == '#' {
            if sep.is_empty() {
                break;
            } else {
                token.push(c);
                continue;
            }
        }
        if c == '\'' || c == '"' || c == '`' {
            if sep.is_empty() {
                sep.push(c);
                token.push(c);
                continue;
            } else if sep == c.to_string() {
                token.push(c);
                sep = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        if c == '&' || c == '|' {
            // needs watch ahead here
            if sep.is_empty() && i + 1 <= len - 1 {
                let c_next = line.chars().nth(i + 1).expect("chars nth error");
                if c_next != c {
                    token.push(c);
                    continue;
                }
            }

            if sep.is_empty() {
                sep.push(c);
                continue;
            } else if c.to_string() == sep {
                result.push(token.trim().to_string());
                token = String::new();
                result.push(format!("{}{}", sep, sep));
                sep = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        if c == ';' {
            if sep.is_empty() {
                result.push(token.trim().to_string());
                result.push(String::from(";"));
                token = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        token.push(c);
    }
    if !token.is_empty() {
        result.push(token.trim().to_string());
    }
    result
}

pub fn parse_args(line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut sep = String::new();
    let mut token = String::new();
    let mut has_backslash = false;
    for c in line.chars() {
        if c == '\\' {
            if !has_backslash {
                has_backslash = true;
            } else {
                has_backslash = false;
                token.push(c);
            }
            continue;
        }
        if c == '#' {
            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }
            if sep.is_empty() {
                break;
            } else {
                token.push(c);
                continue;
            }
        }
        if c == ' ' {
            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }
            if !sep.is_empty() {
                token.push(c);
                continue;
            }
            if token.is_empty() {
                continue;
            } else if sep.is_empty() {
                result.push((String::from(""), token));
                token = String::new();
                continue;
            }
            continue;
        }
        if c == '\'' || c == '"' || c == '`' {
            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }

            if sep == "" {
                sep.push(c);
                continue;
            } else if sep == c.to_string() {
                result.push((c.to_string(), token));
                sep = String::new();
                token = String::new();
                continue;
            } else {
                token.push(c);
            }
        } else {
            if has_backslash {
                has_backslash = false;
            }
            token.push(c);
        }
    }
    if !token.is_empty() {
        result.push((String::from(""), token));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use super::parse_line;
    use super::parse_commands;

    fn _assert_vec_tuple_eq(a: Vec<(String, String)>, b: Vec<(&str, &str)>) {
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            let (ref l, ref r) = *item;
            assert_eq!(l, b[i].0);
            assert_eq!(r, b[i].1);
        }
    }

    fn _assert_vec_str_eq(a: Vec<String>, b: Vec<&str>) {
        println!("a: {:?}", a);
        println!("b: {:?}", b);
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            assert_eq!(item, b[i]);
        }
    }

    #[test]
    fn test_parse_args() {
        let v = vec![
            ("ls", vec![("", "ls")]),
            ("  ls   ", vec![("", "ls")]),
            ("ls -lh", vec![("", "ls"), ("", "-lh")]),
            ("  ls   -lh   ", vec![("", "ls"), ("", "-lh")]),
            ("ls 'abc'", vec![("", "ls"), ("'", "abc")]),
            ("ls \"Hi 你好\"", vec![("", "ls"), ("\"", "Hi 你好")]),
            ("ls \"abc\"", vec![("", "ls"), ("\"", "abc")]),
            ("echo \"hi $USER\"", vec![("", "echo"), ("\"", "hi $USER")]),
            ("echo 'hi $USER'", vec![("", "echo"), ("'", "hi $USER")]),
            ("echo '###'", vec![("", "echo"), ("'", "###")]),
            ("echo a\\ bc", vec![("", "echo"), ("", "a bc")]),
            ("echo \\#", vec![("", "echo"), ("", "#")]),
            ("echo 'hi $USER' |  wc  -l ", vec![("", "echo"),
                                                ("'", "hi $USER"),
                                                ("", "|"),
                                                ("", "wc"),
                                                ("", "-l")]),
            ("echo `uname -m` | wc", vec![("", "echo"),
                                                ("`", "uname -m"),
                                                ("", "|"),
                                                ("", "wc")]),
            ("echo `uname -m` | wc # test it", vec![("", "echo"),
                                                ("`", "uname -m"),
                                                ("", "|"),
                                                ("", "wc")]),
            ("echo '`uname -m`'", vec![("", "echo"), ("'", "`uname -m`")]),
            ("'\"\"\"\"'", vec![("'", "\"\"\"\"")]),
            ("\"\'\'\'\'\"", vec![("\"", "''''")]),
        ];
        for (left, right) in v {
            _assert_vec_tuple_eq(parse_args(left), right);
        }
    }

    #[test]
    fn test_parse_line() {
        let v = vec![
            ("ls", vec!["ls"]),
            ("  ls   ", vec!["ls"]),
            ("ls -lh", vec!["ls", "-lh"]),
            ("ls 'abc'", vec!["ls", "abc"]),
            ("ls a\\ c", vec!["ls", "a c"]),
            ("ls \"abc\"", vec!["ls", "abc"]),
            ("ls \"Hi 你好\"", vec!["ls", "Hi 你好"]),
            ("echo \"hi $USER\"", vec!["echo", "hi $USER"]),
            ("echo 'hi $USER'", vec!["echo", "hi $USER"]),
            ("echo 'hi $USER' |  wc  -l ", vec!["echo", "hi $USER", "|", "wc", "-l"]),
            ("echo `uname -m` | wc", vec!["echo", "uname -m", "|", "wc"]),
            ("echo `uptime` | wc # testing", vec!["echo", "uptime", "|", "wc"]),
            ("awk -F \"[ ,.\\\"]+\"", vec!["awk", "-F", "[ ,.\"]+"]),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(parse_line(left), right);
        }
    }

    #[test]
    fn test_parse_commands() {
        let v = vec![
            ("ls", vec!["ls"]),
            ("ls -lh", vec!["ls -lh"]),
            ("awk -F \" \" '{print $1}' README.md", vec!["awk -F \" \" '{print $1}' README.md"]),
            ("ls | wc", vec!["ls | wc"]),
            ("echo #foo; echo bar", vec!["echo"]),
            ("echo foo; echo bar", vec!["echo foo", ";", "echo bar"]),
            ("echo 'foo; echo bar'", vec!["echo 'foo; echo bar'"]),
            ("echo \"foo; echo bar\"", vec!["echo \"foo; echo bar\""]),
            ("echo `foo; echo bar`", vec!["echo `foo; echo bar`"]),
            ("echo foo && echo bar", vec!["echo foo", "&&", "echo bar"]),
            ("echo foo && echo bar && echo baz",
             vec!["echo foo", "&&", "echo bar", "&&", "echo baz"]),
            ("echo foo || echo bar", vec!["echo foo", "||", "echo bar"]),
            ("echo foo && echo bar; echo end", vec!["echo foo", "&&", "echo bar", ";", "echo end"]),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(parse_commands(left), right);
        }
    }
}