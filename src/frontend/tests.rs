// TODO: Make tests pass

mod wrap_text {
    use crate::frontend::wrap_text;

    #[test]
    fn one_word() {
        let right = vec!["TEST"];
        let left = wrap_text("TEST", 10);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn empty_line() {
        let right = vec![""];
        let left = wrap_text("", 10);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn two_line() {
        let right = vec!["TEST", "TEST"];
        let left = wrap_text("TEST TEST", 5);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn width_too_short() {
        wrap_text("TEST TEST", 3);
        todo!("Make Paragraph::build return a WrapError");
    }

    #[test]
    fn one_word_width_too_short() {
        wrap_text("TEST", 3);
        todo!("Make Paragraph::build return a WrapError");
    }
}

mod paragraph {
    use crate::frontend::{Component, Paragraph};

    #[test]
    fn one_word() {
        let right = vec!["", "    TEST"];
        let left = Paragraph::build("TEST", 10);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn empty() {
        let right = vec!["", ""];
        let left = Paragraph::build("", 10);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn two_line() {
        let right = vec!["", "    TEST", "TEST"];
        let left = Paragraph::build("TEST TEST", 10);
        assert!(left.iter().all(|row| row.chars().count() <= 10));
        assert_eq!(left, right);
    }

    #[test]
    fn cant_wrap() {
        Paragraph::build("TEST", 3);
        todo!("Make Paragraph::build return a WrapError");
    }
}

mod boxed {
    use crate::frontend::{Boxed, Component};

    #[test]
    fn one_word() {
        let right: Vec<String> = r#"
 ┌───────┐ 
 │ TEST  │ 
 └───────┘ 
"#
        .split("\n")
        .map(|l| l.to_string())
        .collect();
        let left = Boxed::build("TEST", 10);
        dbg!(&left);
        assert!(left.iter().all(|row| row.chars().count() == 10));
        assert_eq!(left, right);
    }

    #[test]
    fn empty() {
        let right: Vec<String> = r#"
 ┌───────┐ 
 │       │ 
 └───────┘ 
"#
        .split("\n")
        .map(|l| l.to_string())
        .collect();
        let left = Boxed::build("", 10);
        dbg!(&left);
        assert!(left.iter().all(|row| row.chars().count() == 10));
        assert_eq!(left, right);
    }

    #[test]
    fn two_lines() {
        let right: Vec<String> = r#"
 ┌───────┐ 
 │ TEST  │ 
 │ TEST  │ 
 └───────┘ 
"#
        .split("\n")
        .map(|l| l.to_string())
        .collect();
        let left = Boxed::build("TEST TEST", 10);
        dbg!(&left);
        assert!(left.iter().all(|row| row.chars().count() == 10));
        assert_eq!(left, right);
    }

    #[test]
    fn two_paragraphs() {
        let right: Vec<String> = r#"
 ┌───────┐ 
 │ TEST  │ 
 │       │ 
 │ TEST  │ 
 └───────┘ 
"#
        .split("\n")
        .map(|l| l.to_string())
        .collect();
        let left = Boxed::build("TEST\nTEST", 10);
        dbg!(&left);
        assert!(left.iter().all(|row| row.chars().count() == 10));
        assert_eq!(left, right);
    }

    #[test]
    fn cant_draw_the_box() {
        Boxed::build("asd", 3);
    }

    #[test]
    fn cant_pad_the_box() {
        Boxed::build("asd", 5);
    }

    #[test]
    fn cant_fit_in_the_box() {
        Boxed::build("asd", 7);
    }
}

mod textpad {
    use crate::frontend::{self, Components, TextPad};

    fn setup_textpad() -> TextPad {
        let (w, h) = (10, 10);
        let lorem_ipsum1 = String::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit.");
        let lorem_ipsum2 =
            String::from("Lorem iaculis sem ac magna iaculis, sit amet imperdiet tortor congue.");
        let lorem_ipsum3 =
            String::from("Nam sed auctor metus. Sed viverra neque vitae pharetra dictum.");
        let comps = vec![
            Components::Paragraph(lorem_ipsum1),
            Components::Paragraph(lorem_ipsum2),
            Components::Paragraph(lorem_ipsum3),
        ];
        let body = frontend::build_componenets(comps, w.into());
        TextPad::new(body, h, w).unwrap()
    }

    #[test]
    fn scroll_by_lines() {
        let mut tp = setup_textpad();
        dbg!(&tp.content);
        tp.scroll_by_lines(vec![], 5).unwrap();
        assert_eq!(tp.first, 5);
        tp.scroll_by_lines(vec![], -5).unwrap();
        assert_eq!(tp.first, 0);
    }

    #[test]
    fn scroll_by_paragraph() {
        let mut tp = setup_textpad();
        dbg!(&tp.content);
        tp.scroll_by(vec![], crate::ScrollType::DownByFeedItem)
            .unwrap();
        assert_eq!(tp.first, 8);
        tp.scroll_by(vec![], crate::ScrollType::UpByFeedItem)
            .unwrap();
        assert_eq!(tp.first, 0);
    }
}

// TODO: Write more tests
