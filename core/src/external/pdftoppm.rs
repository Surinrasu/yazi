use std::path::Path;

use adaptor::Image;
use regex::Regex;
use shared::PeekError;
use tokio::process::Command;

pub async fn pdftoppm(src: &Path, dest: impl AsRef<Path>, skip: usize) -> Result<(), PeekError> {
	let output = Command::new("pdftoppm")
		.args(["-singlefile", "-jpeg", "-jpegopt", "quality=75", "-f"])
		.arg((skip + 1).to_string())
		.arg(src)
		.kill_on_drop(true)
		.output()
		.await?;

	if !output.status.success() {
		let s = String::from_utf8_lossy(&output.stderr);
		let pages: usize = Regex::new(r"the last page \((\d+)\)")
			.unwrap()
			.captures(&s)
			.map(|cap| cap[1].parse().unwrap())
			.unwrap_or(0);

		return if pages > 0 { Err(PeekError::Exceed(pages - 1)) } else { Err(s.to_string().into()) };
	}

	Ok(Image::precache_anyway(output.stdout.into(), dest).await?)
}
