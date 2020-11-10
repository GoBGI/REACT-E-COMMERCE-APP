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

#[derive(Debug, Clone)]
pub struct Track {
    pub number: u32,
    pub title: Rc<str>,
    pub performer: Rc<str>,
    pub start: f64,
}

pub fn parse_cue(text: &str) -> Cue {
    let mut commands = generate_commands(text);

    let mut cue = Cue {
        title: Rc::from(""),
        performer: Rc::from(""),
        files: Vec::new(),
    };

    let mut file: Option<File> = None;
    let mut track: Option<Track> = None;

    let cmd_iter = commands.drain(0..commands.len()).filter(|c| c.len() > 1);

    for mut cmd in cmd_iter {
        let mut iter = cmd.drain(0..cmd.len());

        let instr = iter.next().unwrap();
        let arg = iter.next().unwrap();

        match instr.as_ref().as_ref() {
            "TITLE" if file.is_none() => {
                cue.title = arg;
            }
            "PERFORMER" if file.is_none() => {
                cue.performer = arg;
            }

            "TITLE" if track.is_some() => {
                if let Some(t) = track.as_mut() {
                    t.title = arg;
                }
            }
            "PERFORMER" if track.is_some() => {
                if let Some(t) = track.as_mut() {
                    t.performer = arg;
                }
            }

            "FILE" => {
                if let Some(f) = file {
                    cue.files.push(f);
                }

                file = Some(File {
                    path: arg,
                    tracks: Vec::new(),
                });

                track = None;
            }

            "TRACK" if file.is_some() => {
                if let Some(t) = track {
                    if let Some(f) = file.as_mut() {
                        f.tracks.push(t);
                    }
                }

                let number: u32 = arg.parse().unwrap_or(0);

                track = Some(Track {
                    number,
                    title: cue.title.clone(),
                    performer: cue.performer.clone(),
                    start: 0f64,
                });
            }

            "INDEX" if track.is_some() && arg.as_ref() == "01" => {
                if let Some(pos_str) = iter.next() {
                    let parts: V