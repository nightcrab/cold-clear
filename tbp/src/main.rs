fn main() {
    let incoming = futures::stream::repeat_with(|| {
        let mut line = String::new();
        // the frontend closing our input means quit
        if std::io::stdin().read_line(&mut line).unwrap() == 0 {
            return cc_tbp::protocol::FrontendMessage::Quit;
        }
        serde_json::from_str(&line).unwrap()
    });

    let outgoing = futures::sink::unfold((), |_, msg| {
        serde_json::to_writer(std::io::stdout(), &msg).unwrap();
        println!();
        async { Ok(()) }
    });

    futures::pin_mut!(incoming);
    futures::pin_mut!(outgoing);

    futures::executor::block_on(cc_tbp::run(incoming, outgoing));
}
