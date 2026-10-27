use crate::structure::{
    ElementKind, FlagsKind, KeyEncoding, KeyMatcher, KeySpec, LayerState, Presence, StructureNode,
};
use dpp::util::deserializer::ProtocolVersion;

impl StructureNode {
    fn new(segment: &str, key: KeySpec) -> Self {
        StructureNode {
            id: String::new(),
            segment: segment.to_string(),
            key,
            kinds: vec![],
            kinds_note: None,
            flags: vec![FlagsKind::None],
            flags_note: None,
            value: None,
            reference: None,
            since: 1,
            until: None,
            presence: Presence::Always,
            source: String::new(),
            book: None,
            description: String::new(),
            recurse: None,
            opaque: None,
            states: vec![],
            children: vec![],
        }
    }

    /// The root of the structure
    pub fn root() -> Self {
        let mut node = Self::new("root", KeySpec::Root);
        node.kinds = vec![ElementKind::Tree];
        node
    }

    /// A node at a fixed key. `constant` names the Rust constant the bytes
    /// come from and is empty when the code uses a literal.
    pub fn fixed(segment: &str, bytes: &[u8], label: &str, constant: &str) -> Self {
        Self::new(
            segment,
            KeySpec::Fixed {
                bytes: bytes.to_vec(),
                label: label.to_string(),
                constant: constant.to_string(),
                ascii: false,
            },
        )
    }

    /// The code writes this fixed key as a character, such as `b"s"`
    pub fn ascii(mut self) -> Self {
        if let KeySpec::Fixed { ascii, .. } = &mut self.key {
            *ascii = true;
        }
        self
    }

    /// A template standing for many keys
    pub fn dynamic(
        segment: &str,
        name: &str,
        matcher: KeyMatcher,
        encoding: KeyEncoding,
        description: &str,
    ) -> Self {
        Self::new(
            segment,
            KeySpec::Dynamic {
                name: name.to_string(),
                matcher,
                encoding,
                description: description.to_string(),
            },
        )
    }

    /// A template for 32 byte identifiers
    pub fn identifier(segment: &str, name: &str, description: &str) -> Self {
        Self::dynamic(
            segment,
            name,
            KeyMatcher::Len(32),
            KeyEncoding::Identifier32,
            description,
        )
    }

    /// The single element kind at this key
    pub fn kind(mut self, kind: ElementKind) -> Self {
        self.kinds = vec![kind];
        self
    }

    /// The element kinds the code chooses between, with what decides
    pub fn kinds(mut self, kinds: &[ElementKind], note: &str) -> Self {
        self.kinds = kinds.to_vec();
        self.kinds_note = Some(note.to_string());
        self
    }

    /// The element flags on the element. `note` says who the owner in the
    /// flags is, or what decides between several kinds.
    pub fn flags(mut self, flags: &[FlagsKind], note: &str) -> Self {
        self.flags = flags.to_vec();
        self.flags_note = Some(note.to_string());
        self
    }

    /// What an item holds
    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    /// For references, the identifier of the node they point to
    pub fn reference(mut self, target: &str) -> Self {
        self.reference = Some(target.to_string());
        self
    }

    /// The first protocol version the node exists in. Children inherit it
    /// unless they name a later one.
    pub fn since(mut self, protocol_version: ProtocolVersion) -> Self {
        self.since = protocol_version;
        self
    }

    /// The last protocol version the node exists in
    pub fn until(mut self, protocol_version: ProtocolVersion) -> Self {
        self.until = Some(protocol_version);
        self
    }

    /// The node is created on first use rather than with its parent
    pub fn lazy(mut self) -> Self {
        self.presence = Presence::Lazy;
        self
    }

    /// The node is created with its parent and deleted later, while the
    /// parent stays
    pub fn until_deleted(mut self) -> Self {
        self.presence = Presence::UntilDeleted;
        self
    }

    /// The repository relative file holding the canonical definition.
    /// Children inherit it unless they name their own.
    pub fn source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    /// The book chapter describing this part, relative to `book/src`
    pub fn book(mut self, chapter: &str) -> Self {
        self.book = Some(chapter.to_string());
        self
    }

    /// What the node is for
    pub fn describe(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// The node's children are those of the node with this identifier
    pub fn recurse(mut self, target: &str) -> Self {
        self.recurse = Some(target.to_string());
        self
    }

    /// The layer below is not a Merk of elements; says what it holds
    pub fn opaque(mut self, contents: &str) -> Self {
        self.opaque = Some(contents.to_string());
        self
    }

    /// Adds a state the layer below goes through. States are listed in the
    /// order the layer goes through them; `keys` are the segments of the
    /// children the layer holds in that state.
    pub fn state(mut self, name: &str, title: &str, description: &str, keys: &[&str]) -> Self {
        self.states.push(LayerState {
            name: name.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            keys: keys.iter().map(|key| key.to_string()).collect(),
        });
        self
    }

    /// Adds one child
    pub fn child(mut self, child: StructureNode) -> Self {
        self.children.push(child);
        self
    }

    /// Adds several children
    pub fn children(mut self, children: Vec<StructureNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// Finishes the tree: computes identifiers, lets children inherit
    /// `since` and `source`, and orders each layer (fixed keys by their
    /// bytes, as GroveDB orders them, then templates in declaration order).
    pub fn build(mut self) -> Self {
        self.id = self.segment.clone();
        self.finish_children();
        self
    }

    fn finish_children(&mut self) {
        let (mut fixed, dynamic): (Vec<_>, Vec<_>) = std::mem::take(&mut self.children)
            .into_iter()
            .partition(|child| matches!(child.key, KeySpec::Fixed { .. }));
        fixed.sort_by(|a, b| a.fixed_key_bytes().cmp(&b.fixed_key_bytes()));
        self.children = fixed.into_iter().chain(dynamic).collect();

        let is_root = matches!(self.key, KeySpec::Root);
        for child in self.children.iter_mut() {
            child.id = if is_root {
                child.segment.clone()
            } else {
                format!("{}.{}", self.id, child.segment)
            };
            child.since = child.since.max(self.since);
            if child.source.is_empty() {
                child.source = self.source.clone();
            }
            child.finish_children();
        }
    }

    /// The bytes of a fixed key
    pub fn fixed_key_bytes(&self) -> Option<&[u8]> {
        match &self.key {
            KeySpec::Fixed { bytes, .. } => Some(bytes.as_slice()),
            KeySpec::Root | KeySpec::Dynamic { .. } => None,
        }
    }

    /// Whether the node exists in this protocol version
    pub fn exists_in(&self, protocol_version: ProtocolVersion) -> bool {
        self.since <= protocol_version && self.until.is_none_or(|until| protocol_version <= until)
    }

    /// Visits the node and everything below it, parents first
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a StructureNode)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }

    /// Finds a node by identifier
    pub fn find(&self, id: &str) -> Option<&StructureNode> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }
}
