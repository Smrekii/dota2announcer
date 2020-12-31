use crate::embed::Disposition::{Attachment, Inline};
use rocket::handler::{Handler, Outcome};
use rocket::http::ext::IntoOwned;
use rocket::http::uri::Segments;
use rocket::http::{ContentType, Header, Method};
use rocket::response::{Redirect, Responder};
use rocket::{response, Data, Request, Route};
use rocket_contrib::serve::Options;
use rust_embed::RustEmbed;
use serde::export::PhantomData;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct EmbedFiles<T: RustEmbed + Clone + Send + Sync + 'static> {
    phantom: PhantomData<T>,
    root: PathBuf,
    options: Options,
    rank: isize,
}

impl<T: RustEmbed + Clone + Send + Sync + 'static> EmbedFiles<T> {
    const DEFAULT_RANK: isize = 10;

    pub fn new(options: Options) -> Self {
        EmbedFiles {
            phantom: PhantomData,
            root: Path::new("").to_path_buf(),
            options,
            rank: Self::DEFAULT_RANK,
        }
    }

    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }
}

impl<T: RustEmbed + Clone + Send + Sync + 'static> Into<Vec<Route>> for EmbedFiles<T> {
    fn into(self) -> Vec<Route> {
        let non_index = Route::ranked(self.rank, Method::Get, "/<path..>", self.clone());
        // `Index` requires routing the index for obvious reasons.
        // `NormalizeDirs` requires routing the index so a `.mount("/foo")` with
        // a request `/foo`, can be redirected to `/foo/`.
        if self.options.contains(Options::Index) || self.options.contains(Options::NormalizeDirs) {
            let index = Route::ranked(self.rank, Method::Get, "/", self);
            vec![index, non_index]
        } else {
            vec![non_index]
        }
    }
}

async fn handle_dir<'r, P, T>(
    files: &EmbedFiles<T>,
    r: &'r Request<'_>,
    d: Data,
    p: P,
) -> Outcome<'r>
where
    P: AsRef<Path>,
    T: RustEmbed + Clone + Send + Sync + 'static,
{
    if files.options.contains(Options::NormalizeDirs) && !r.uri().path().ends_with('/') {
        let new_path = r
            .uri()
            .map_path(|p| p.to_owned() + "/")
            .expect("adding a trailing slash to a known good path results in a valid path")
            .into_owned();

        return Outcome::from_or_forward(r, d, Redirect::permanent(new_path));
    }

    if !files.options.contains(Options::Index) {
        return Outcome::forward(d);
    }

    let file = <EmbedFile<T>>::of(p.as_ref().join("index.html")).ok();
    Outcome::from_or_forward(r, d, file)
}

#[rocket::async_trait]
impl<T: RustEmbed + Clone + Send + Sync + 'static> Handler for EmbedFiles<T> {
    async fn handle<'r, 's: 'r>(&'s self, req: &'r Request<'_>, data: Data) -> Outcome<'r> {
        // If this is not the route with segments, handle it only if the user
        // requested a handling of index files.
        let current_route = req.route().expect("route while handling");
        let is_segments_route = current_route.uri.path().ends_with(">");
        if !is_segments_route {
            return handle_dir(self, req, data, &self.root).await;
        }

        // Otherwise, we're handling segments. Get the segments as a `PathBuf`,
        // only allowing dotfiles if the user allowed it.
        let allow_dotfiles = self.options.contains(Options::DotFiles);
        let path = req
            .get_segments::<Segments<'_>>(0)
            .and_then(|res| res.ok())
            .and_then(|segments| segments.into_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path));

        match path {
            Some(p) if p.is_dir() => handle_dir(self, req, data, p).await,
            Some(p) => Outcome::from_or_forward(req, data, <EmbedFile<T>>::of(p).ok()),
            None => Outcome::forward(data),
        }
    }
}
pub enum Disposition {
    Attachment,
    Inline,
}

impl Disposition {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Attachment => "attachment",
            Self::Inline => "inline",
        }
    }
}

pub struct EmbedFile<T: RustEmbed> {
    path: PathBuf,
    disposition: Option<Disposition>,
    phantom: PhantomData<T>,
}

impl<T: RustEmbed> EmbedFile<T> {
    pub fn of<P: AsRef<Path>>(path: P) -> io::Result<EmbedFile<T>> {
        Self::new(path, None)
    }

    pub fn of_attachment<P: AsRef<Path>>(path: P) -> io::Result<EmbedFile<T>> {
        Self::new(path, Some(Attachment))
    }

    pub fn of_inline<P: AsRef<Path>>(path: P) -> io::Result<EmbedFile<T>> {
        Self::new(path, Some(Inline))
    }

    pub fn new<P: AsRef<Path>>(
        path: P,
        disposition: Option<Disposition>,
    ) -> io::Result<EmbedFile<T>> {
        Ok(EmbedFile {
            path: path.as_ref().to_path_buf(),
            disposition,
            phantom: PhantomData,
        })
    }
}

impl<'r, T: RustEmbed> Responder<'r, 'static> for EmbedFile<T> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let file = T::get(&self.path.to_string_lossy());

        let mut response = file.respond_to(req)?;
        if let Some(ext) = self.path.extension() {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }

        if let Some(disposition) = self.disposition {
            if let Some(name) = self.path.file_name() {
                response.set_header(Header::new(
                    "Content-Disposition",
                    format!(
                        "{}; filename=\"{}\"",
                        disposition.to_str(),
                        name.to_string_lossy()
                    ),
                ));
            }
        }

        Ok(response)
    }
}
