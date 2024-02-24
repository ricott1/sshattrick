use ratatui::{prelude::*, widgets::Paragraph};

fn big_text(text: Vec<&str>, color_1: Color, color_2: Color) -> Paragraph {
    let lines = text
        .iter()
        .map(|line| {
            let mut spans = vec![];
            for c in line.chars() {
                if c == '█' {
                    spans.push(Span::styled("█", color_1));
                } else {
                    spans.push(Span::styled(c.to_string(), color_2));
                }
            }
            Line::from(spans)
        })
        .collect::<Vec<Line>>();
    Paragraph::new(lines).alignment(Alignment::Center)
}

fn big_text_from_string<'a>(text: Vec<String>, color_1: Color, color_2: Color) -> Paragraph<'a> {
    let lines = text
        .iter()
        .map(|line| {
            let mut spans = vec![];
            for c in line.chars() {
                if c == '█' {
                    spans.push(Span::styled("█", color_1));
                } else {
                    spans.push(Span::styled(c.to_string(), color_2));
                }
            }
            Line::from(spans)
        })
        .collect::<Vec<Line>>();
    Paragraph::new(lines).alignment(Alignment::Center)
}

pub fn dots(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec!["   ", "██╗", "╚═╝", "██╗", "╚═╝", "   "],
        color_1,
        color_2,
    )
}

pub fn hyphen(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec!["     ", "     ", "████╗", "╚═══╝", "     ", "     "],
        color_1,
        color_2,
    )
}

pub fn zero<'a>() -> Vec<&'a str> {
    vec![
        " ██████╗ ",
        "██╔═████╗",
        "██║██╔██║",
        "████╔╝██║",
        "╚██████╔╝",
        " ╚═════╝ ",
    ]
}

pub fn one<'a>() -> Vec<&'a str> {
    vec![" ██╗", "███║", "╚██║", " ██║", " ██║", " ╚═╝"]
}

pub fn two<'a>() -> Vec<&'a str> {
    vec![
        "██████╗ ",
        "╚════██╗",
        " █████╔╝",
        "██╔═══╝ ",
        "███████╗",
        "╚══════╝",
    ]
}

pub fn three<'a>() -> Vec<&'a str> {
    vec![
        "██████╗ ",
        "╚════██╗",
        " █████╔╝",
        " ╚═══██╗",
        "██████╔╝",
        "╚═════╝ ",
    ]
}

pub fn four<'a>() -> Vec<&'a str> {
    vec![
        "██╗  ██╗",
        "██║  ██║",
        "███████║",
        "╚════██║",
        "     ██║",
        "     ╚═╝",
    ]
}

pub fn five<'a>() -> Vec<&'a str> {
    vec![
        "███████╗",
        "██╔════╝",
        "███████╗",
        "╚════██║",
        "███████║",
        "╚══════╝",
    ]
}

pub fn six<'a>() -> Vec<&'a str> {
    vec![
        " ██████╗ ",
        "██╔════╝ ",
        "███████╗ ",
        "██╔═══██╗",
        "╚██████╔╝",
        " ╚═════╝ ",
    ]
}

pub fn seven<'a>() -> Vec<&'a str> {
    vec![
        "███████╗",
        "╚════██║",
        "    ██╔╝",
        "   ██╔╝ ",
        "   ██║  ",
        "   ╚═╝  ",
    ]
}

pub fn eight<'a>() -> Vec<&'a str> {
    vec![
        " █████╗ ",
        "██╔══██╗",
        "╚█████╔╝",
        "██╔══██╗",
        "╚█████╔╝",
        " ╚════╝ ",
    ]
}

pub fn nine<'a>() -> Vec<&'a str> {
    vec![
        " █████╗ ",
        "██╔══██╗",
        "╚██████║",
        " ╚═══██║",
        " █████╔╝",
        " ╚════╝ ",
    ]
}

fn digit_to_str<'a>(x: u8) -> Vec<&'a str> {
    match x {
        0 => zero(),
        1 => one(),
        2 => two(),
        3 => three(),
        4 => four(),
        5 => five(),
        6 => six(),
        7 => seven(),
        8 => eight(),
        9 => nine(),
        _ => panic!("Invalid digit"),
    }
}

pub trait BigNumberFont {
    fn big_font(&self) -> Paragraph<'static>;
    fn big_font_styled(&self, color_1: Color, color_2: Color) -> Paragraph<'static>;
}
impl BigNumberFont for u8 {
    fn big_font(&self) -> Paragraph<'static> {
        match self {
            x if x.clone() < 10 => big_text(digit_to_str(x.clone()), Color::Cyan, Color::White),
            x if x.clone() < 100 => {
                let tens = digit_to_str(x / 10);
                let units = digit_to_str(x % 10);
                let mut total = vec![];
                for idx in 0..6 {
                    total.push(tens[idx].to_string() + units[idx]);
                }
                big_text_from_string(total, Color::Cyan, Color::White)
            }
            _ => dots(Color::Cyan, Color::White),
        }
    }
    fn big_font_styled(&self, color_1: Color, color_2: Color) -> Paragraph<'static> {
        match self {
            x if x.clone() < 10 => big_text(digit_to_str(x.clone()), color_1, color_2),
            x if x.clone() < 100 => {
                let tens = digit_to_str(x / 10);
                let units = digit_to_str(x % 10);
                let mut total = vec![];
                for idx in 0..6 {
                    total.push(tens[idx].to_string() + units[idx]);
                }
                big_text_from_string(total, color_1, color_2)
            }
            _ => dots(color_1, color_2),
        }
    }
}

pub fn red_scored(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec![
            "██████╗ ███████╗██████╗     ███████╗ ██████╗ ██████╗ ██████╗ ███████╗██████╗ ██╗",
            "██╔══██╗██╔════╝██╔══██╗    ██╔════╝██╔════╝██╔═══██╗██╔══██╗██╔════╝██╔══██╗██║",
            "██████╔╝█████╗  ██║  ██║    ███████╗██║     ██║   ██║██████╔╝█████╗  ██║  ██║██║",
            "██╔══██╗██╔══╝  ██║  ██║    ╚════██║██║     ██║   ██║██╔══██╗██╔══╝  ██║  ██║╚═╝",
            "██║  ██║███████╗██████╔╝    ███████║╚██████╗╚██████╔╝██║  ██║███████╗██████╔╝██╗",
            "╚═╝  ╚═╝╚══════╝╚═════╝     ╚══════╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚═╝",
        ],
        color_1,
        color_2,
    )
}

pub fn blue_scored(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec![
        "██████╗ ██╗     ██╗   ██╗███████╗    ███████╗ ██████╗ ██████╗ ██████╗ ███████╗██████╗ ██╗",
        "██╔══██╗██║     ██║   ██║██╔════╝    ██╔════╝██╔════╝██╔═══██╗██╔══██╗██╔════╝██╔══██╗██║",
        "██████╔╝██║     ██║   ██║█████╗      ███████╗██║     ██║   ██║██████╔╝█████╗  ██║  ██║██║",
        "██╔══██╗██║     ██║   ██║██╔══╝      ╚════██║██║     ██║   ██║██╔══██╗██╔══╝  ██║  ██║╚═╝",
        "██████╔╝███████╗╚██████╔╝███████╗    ███████║╚██████╗╚██████╔╝██║  ██║███████╗██████╔╝██╗",
        "╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝    ╚══════╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ╚═╝",
    ],
        color_1,
        color_2,
    )
}

pub fn red_won(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec![
            "██████╗ ███████╗██████╗     ██╗    ██╗ ██████╗ ███╗   ██╗██╗",
            "██╔══██╗██╔════╝██╔══██╗    ██║    ██║██╔═══██╗████╗  ██║██║",
            "██████╔╝█████╗  ██║  ██║    ██║ █╗ ██║██║   ██║██╔██╗ ██║██║",
            "██╔══██╗██╔══╝  ██║  ██║    ██║███╗██║██║   ██║██║╚██╗██║╚═╝",
            "██║  ██║███████╗██████╔╝    ╚███╔███╔╝╚██████╔╝██║ ╚████║██╗",
            "╚═╝  ╚═╝╚══════╝╚═════╝      ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═══╝╚═╝",
        ],
        color_1,
        color_2,
    )
}

pub fn blue_won(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec![
            "██████╗ ██╗     ██╗   ██╗███████╗    ██╗    ██╗ ██████╗ ███╗   ██╗██╗",
            "██╔══██╗██║     ██║   ██║██╔════╝    ██║    ██║██╔═══██╗████╗  ██║██║",
            "██████╔╝██║     ██║   ██║█████╗      ██║ █╗ ██║██║   ██║██╔██╗ ██║██║",
            "██╔══██╗██║     ██║   ██║██╔══╝      ██║███╗██║██║   ██║██║╚██╗██║╚═╝",
            "██████╔╝███████╗╚██████╔╝███████╗    ╚███╔███╔╝╚██████╔╝██║ ╚████║██╗",
            "╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝     ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═══╝╚═╝",
        ],
        color_1,
        color_2,
    )
}

pub fn draw(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(
        vec![
            "██████╗ ██████╗  █████╗ ██╗    ██╗██╗",
            "██╔══██╗██╔══██╗██╔══██╗██║    ██║██║",
            "██║  ██║██████╔╝███████║██║ █╗ ██║██║",
            "██║  ██║██╔══██╗██╔══██║██║███╗██║╚═╝",
            "██████╔╝██║  ██║██║  ██║╚███╔███╔╝██╗",
            "╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚═╝",
        ],
        color_1,
        color_2,
    )
}

pub fn disconnection(color_1: Color, color_2: Color) -> Paragraph<'static> {
    big_text(vec![
"██████╗ ██╗███████╗ ██████╗ ██████╗ ███╗   ██╗███╗   ██╗███████╗ ██████╗████████╗██╗ ██████╗ ███╗   ██╗",
"██╔══██╗██║██╔════╝██╔════╝██╔═══██╗████╗  ██║████╗  ██║██╔════╝██╔════╝╚══██╔══╝██║██╔═══██╗████╗  ██║",
"██║  ██║██║███████╗██║     ██║   ██║██╔██╗ ██║██╔██╗ ██║█████╗  ██║        ██║   ██║██║   ██║██╔██╗ ██║",
"██║  ██║██║╚════██║██║     ██║   ██║██║╚██╗██║██║╚██╗██║██╔══╝  ██║        ██║   ██║██║   ██║██║╚██╗██║",
"██████╔╝██║███████║╚██████╗╚██████╔╝██║ ╚████║██║ ╚████║███████╗╚██████╗   ██║   ██║╚██████╔╝██║ ╚████║",
"╚═════╝ ╚═╝╚══════╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝  ╚═══╝╚══════╝ ╚═════╝   ╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═══╝"
],
color_1,
color_2
)
}
