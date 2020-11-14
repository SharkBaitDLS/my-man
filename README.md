### Building
`cargo build [--release]`

Note: this depends on [serenity-rs](https://github.com/serenity-rs/serenity) which in turn depends on [audiopus](https://crates.io/crates/audiopus) and [sodiumoxide](https://crates.io/crates/sodiumoxide). On some platforms those may need to have their dependent libraries manually built and linked rather than built by the crates themselves. Consult the crate docs for those packages for more information on setting up your build environment.

### Running
#### Runtime Prerequisites
* [ffmpeg](https://ffmpeg.org/download.html)
* [youtube-dl](https://ytdl-org.github.io/youtube-dl/download.html)

#### Execution
Run the binary with the following environment variables set:
* `DISCORD_TOKEN`: your bot token from Discord
* `AUDIO_FILE_DIR`: the local directory path to read MP3 files from

### Usage
When a user joins a voice channel in its server, the bot will look in `AUDIO_FILE_DIR` for an mp3 file matching the user's Discord username in all lowercase to play. If provided, it will play `myman.mp3` to announce itself when it rejoins a channel after being orphaned in another one. It accepts direct commands to play audio files as well, type `?help` in a server chatroom the bot can read to see the available commands.
