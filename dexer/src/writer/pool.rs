/// Interned, sorted string pool.  Returns the index of a string after insertion.
#[derive(Default)]
pub struct StringPool {
    strings: Vec<String>,
}

impl StringPool {
    /// Insert if absent; return stable index (pre-sort).  Call `sort` before using indices.
    pub fn intern(&mut self, s: impl Into<String>) -> () {
        let s = s.into();
        if !self.strings.contains(&s) {
            self.strings.push(s);
        }
    }

    /// Sort strings and freeze. Returns sorted vec for index lookup.
    pub fn sorted(mut self) -> SortedStrings {
        self.strings.sort();
        SortedStrings(self.strings)
    }
}

pub struct SortedStrings(pub Vec<String>);

impl SortedStrings {
    pub fn index_of(&self, s: &str) -> u32 {
        self.0
            .binary_search_by(|x| x.as_str().cmp(s))
            .unwrap_or_else(|_| panic!("string not interned: {s:?}")) as u32
    }

    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }
}

// --------------------------------------------------------------------------
// Descriptor helpers
// --------------------------------------------------------------------------

/// Parse a method descriptor's parameter list into individual descriptors.
/// E.g. "(Landroid/content/Context;I)V" → ["Landroid/content/Context;", "I"]
pub fn parse_param_descriptors(descriptor: &str) -> Vec<String> {
    let inner = descriptor
        .trim_start_matches('(')
        .split(')')
        .next()
        .unwrap_or("");
    let mut params = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'L' => {
                let mut cls = String::from("L");
                for ch in chars.by_ref() {
                    cls.push(ch);
                    if ch == ';' {
                        break;
                    }
                }
                params.push(cls);
            }
            '[' => {
                // Array — consume the element descriptor too
                let mut arr = String::from("[");
                if let Some(&next) = chars.peek() {
                    if next == 'L' {
                        chars.next();
                        arr.push('L');
                        for ch in chars.by_ref() {
                            arr.push(ch);
                            if ch == ';' {
                                break;
                            }
                        }
                    } else {
                        arr.push(chars.next().unwrap());
                    }
                }
                params.push(arr);
            }
            c => params.push(c.to_string()),
        }
    }
    params
}

/// Extract the return descriptor from a method descriptor string.
/// E.g. "(II)Ljava/lang/Object;" → "Ljava/lang/Object;"
pub fn return_descriptor(descriptor: &str) -> &str {
    descriptor.split(')').nth(1).unwrap_or("V")
}

/// Build the "shorty" descriptor used in proto_ids.
/// E.g. "(Landroid/content/Context;I)V" → "VLI"
pub fn shorty(descriptor: &str) -> String {
    let ret = return_descriptor(descriptor);
    let ret_short = type_to_shorty(ret);
    let params = parse_param_descriptors(descriptor);
    let mut s = String::new();
    s.push(ret_short);
    for p in &params {
        s.push(type_to_shorty(p));
    }
    s
}

fn type_to_shorty(t: &str) -> char {
    match t.chars().next().unwrap_or('V') {
        'L' | '[' => 'L',
        c => c,
    }
}

/// Build the BYO-constructor descriptor by appending `J` to the arg list.
/// E.g. "(Landroid/content/Context;)V" → "(Landroid/content/Context;J)V"
pub fn byo_descriptor(base_descriptor: &str) -> String {
    let params: String = base_descriptor
        .trim_start_matches('(')
        .splitn(2, ')')
        .next()
        .unwrap_or("")
        .to_string();
    format!("({params}J)V")
}
