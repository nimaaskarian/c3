#[derive(Debug)]
pub(super) struct Search {
    indices: Vec<usize>,
    search_index: usize,
    last_query: String,
}

impl Search {
    #[inline]
    pub fn new() -> Self{
        Search {
            last_query: String::new(),
            indices: vec![],
            search_index: 0,
        }
    }

    #[inline]
    pub fn first_greater_than(&mut self, index: usize) -> Option<usize> {
        if self.indices.is_empty() {
            return None;
        }
        for i in self.indices.clone() {
            if i > index{
                return Some(i);
            }
        }

        Some(self.indices[0])
    }

    #[inline]
    pub fn search(&mut self, needle:Option<String>, haystack: Vec<String>) {
        if let Some(query) = needle {
            self.last_query = query;
        }
        if self.last_query.is_empty() {
            return;
        }
        self.indices = Vec::new();

        // TODO: implement fuzzy matching 
        for i in 0..haystack.len() {
            if haystack[i].contains(self.last_query.as_str()) || haystack[i].to_lowercase().contains(self.last_query.as_str()) {
                self.indices.push(i);
            }
        }
    }

    #[inline]
    pub fn indices(&self) -> Vec<usize>{
        self.indices.clone()
    }

    #[inline]
    pub fn next(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            return None;
        }
        if self.search_index+1 < self.indices.len() {
            self.search_index+=1
        } else {
            self.search_index=0
        }
        Some(self.indices[self.search_index])
    }

    #[inline]
    pub fn prev(&mut self) -> Option<usize> {
        if self.indices.is_empty() {
            return None;
        }
        if self.search_index != 0 {
            self.search_index-=1
        } else {
            self.search_index=self.indices.len()-1
        }
        Some(self.indices[self.search_index])
    }
}
