use std::rc::Rc;

fn generate_commands(text: &str) -> Vec<Vec<Rc<str>>> {
    let iter = text.chars().peekable();
    let mut commands: Vec<Vec<Rc<str>>> = Vec::new();

    let mut quote_delimited = false;
    let mut command: Vec<Rc<str>> = Vec::new();
    let mut string = String::new();

    for ch in iter {
        if ch == '\n' || (!quote_delimited && ch == ' ') || (quote_delimited && ch == '"') {
            if !string.is_empty() || quote_delimited {
                command.push(Rc::from(string));
                string = String::new();
                quote_delimited = false;
            }

            if ch == '\n' {
                commands.push(command);
                command = Vec::new();
            }
        } else if ch == '"' {
            quote_delimited = true;
            continue;
        } else if quote_delimited
            || (ch >= 'A' && ch <= 'Z')
            || (ch >= 'a' && ch <= 'z')
            || (ch >= '0' && ch <= '9')
            || ch == ':'
        {
            string.push(ch);
        }
    }

    if !string.is_empty() {
        command.push(Rc::from(string));
    }

    commands.push(command);

    commands
}

#[derive(Debug, Clone)]
pub struct Cue {
    pub title: Rc<str>,
    pub performer: Rc<str>,
    pub files: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct File {
    pub path: Rc<str>,
    pub tracks: Vec<Track>,
}

