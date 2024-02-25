# ssHattrick

![Watch the demo](./demo.gif)

ssHattrick is a multiplayer game that you can play over SSH. It is a clone of the popular game [Hattrick](https://www.retrogames.cz/play_1368-Atari7800.php).

## Just play!

`ssh frittura.org -p 2020`

Remember to set the terminal to a minimum size of 160x50. Some terminals don't support the game colors, so you might need to try different ones. Here is a list of tested terminals:

-   Linux: whatever the default terminal is, it should work
-   MacOs: [iTerm2](https://iterm2.com/)
-   Windows: need someone to test it

## Build and Run

My server will probably be down very often :) so you might want to run the server yourself.

You need to have the rust toolchain installed --> https://www.rust-lang.org/tools/install. Then you can build the game with

`cargo build --release`

To run the server, you can run the executable and pass the port as an argument (2020 is the default port)

`./target/release/sshattrick -p 2020`

## Contribution

It is almost guaranteed that you will encounter bugs along your journey. If you do, please open an issue and describe what happened. If you are a developer and want to contribute, feel free to open a pull request.

## License

This software is released under the [GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) license.
