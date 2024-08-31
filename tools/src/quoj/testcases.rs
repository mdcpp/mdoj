use std::io::{Cursor, Read, Seek};

use anyhow::Result;
use reqwest::{Client, Url};
use zip::ZipArchive;

pub async fn testcases(
    client: &Client,
    base_url: &Url,
    id: u64,
) -> Result<Testcases<impl Read + Seek>> {
    let bytes = Cursor::new(
        client
            .get(base_url.join("admin/test_case")?)
            .query(&[("problem_id", id)])
            .send()
            .await?
            .bytes()
            .await?,
    );
    let testcases = ZipArchive::new(bytes)?;
    Ok(Testcases(testcases))
}

pub struct Testcases<T: Read + Seek>(ZipArchive<T>);

impl<T: Read + Seek> Testcases<T> {
    pub fn testcase(&mut self, name: impl AsRef<str>) -> Result<Vec<u8>> {
        let mut buf = vec![];
        let mut file = self.0.by_name(name.as_ref())?;
        buf.reserve_exact(file.size() as usize);
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
