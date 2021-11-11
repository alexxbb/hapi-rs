impl std::cmp::PartialEq for super::bindings::HAPI_Session {
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.id == other.id
    }
}
