use fframes::{cli, EncoderOptions, RenderOptions, StaticMediaProvider};
use fframes::cli::clap;
use portal2_music_video::{Portal2MusicVideo, Portal2MusicVideoMedia};
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
struct VideoArgs {
    #[arg(long, default_value = "Aperture: The Test Is Over", global = true)]
    title: String,
}

fn main() -> ExitCode {
    let args = cli::parse::<VideoArgs>();
    let media = Portal2MusicVideoMedia::prepare().expect("media");
    let video = Portal2MusicVideo::new(&media, &args.app.title);
    cli::new(
        &video,
        RenderOptions {
            media: Some(&media),
            load_system_fonts: true,
            video_encoder_options: EncoderOptions {
                preferred_encoder: Some("libx264"),
                codec_params: Some(&[("crf", "19"), ("preset", "medium"), ("tune", "animation")]),
                ..Default::default()
            },
            ..Default::default()
        },
    ).args(args).run()
}
