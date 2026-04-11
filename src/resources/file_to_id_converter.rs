use crate::resources::identifier::Identifier;

struct FileToIdConverter {
    prefix: String,
    extension: String,
}

impl FileToIdConverter {
    pub fn new_json(prefix: String) -> Self {
        FileToIdConverter {
            prefix,
            extension: "json".into(),
        }
    }

    pub fn new(prefix: String, extension: String) -> Self {
        FileToIdConverter { prefix, extension }
    }

    pub fn id_to_file(&self, id: Identifier) -> Option<Identifier> {
        id.with_path(format!(
            "{}/{}{}",
            &self.prefix,
            id.get_path(),
            &self.extension
        ))
        .ok()
    }
}
