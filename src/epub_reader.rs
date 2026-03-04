use anyhow::{Context, Result};
use epub::doc::EpubDoc;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub content: String,
    pub spine_index: usize,
}

#[derive(Debug)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub path: PathBuf,
    pub chapters: Vec<Chapter>,
}

impl Book {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut doc = EpubDoc::new(&path)
            .with_context(|| format!("Failed to open EPUB: {}", path.display()))?;

        let title = doc
            .mdata("title")
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "Unknown Title".to_string());

        let author = doc
            .mdata("creator")
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "Unknown Author".to_string());

        let spine_len = doc.spine.len();
        let mut chapters = Vec::new();

        for idx in 0..spine_len {
            doc.set_current_chapter(idx);

            let chapter_title = doc
                .toc
                .iter()
                .find(|nav| nav.play_order == Some(idx + 1))
                .map(|nav| nav.label.clone())
                .unwrap_or_else(|| format!("Chapter {}", idx + 1));

            let raw_html = match doc.get_current_str() {
                Some((html, _mime)) => html,
                None => continue,
            };

            let plain_text = html_to_text(&raw_html);

            if plain_text.trim().is_empty() {
                continue;
            }

            chapters.push(Chapter {
                title: chapter_title,
                content: plain_text,
                spine_index: idx,
            });
        }

        if chapters.is_empty() {
            anyhow::bail!("No readable content found in EPUB");
        }

        Ok(Book { title, author, path, chapters })
    }
}

fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 10000)
        .lines()
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
