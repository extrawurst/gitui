use git2::Status;

///
pub fn on_index(s:&Status)->bool{
    s.is_index_new() || s.is_index_modified()
}
