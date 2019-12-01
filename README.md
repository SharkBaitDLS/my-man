### Building
`cargo build [--release]`

Note: this depends on [serenity-rs](https://github.com/serenity-rs/serenity) which will attempt to build `libopus` and `libsodium` for you, but on some systems those may need to be manually built.

### Running
#### Runtime Prerequisites
* ffmpeg
* youtube-dl

#### Execution
Run the binary with the following environment variables set:
* `DISCORD_TOKEN`: your bot token from Discord
* `AUDIO_FILE_DIR`: the local directory path to read MP3 files from
