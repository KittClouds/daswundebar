//! RelationSchema - Type-Safe Relation Definitions
//!
//! A compile-time verifiable schema system for narrative relations.
//! Uses enums instead of strings for type safety, bitflags for kind sets,
//! and explicit slot specifications for arity.
//!
//! # Design
//! - `Relation` enum: all relation types, including inverses
//! - `Kind` bitflags: entity type constraints
//! - `SlotSpec`: role + allowed kinds per argument position
//! - `validate_schemas()`: startup verification of consistency

use std::collections::HashMap;

// =============================================================================
// Kind (Entity Types) - Bitflags for efficient set operations
// =============================================================================

/// Entity type flags - combinable for "one of these types" constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Kind {
    Any       = 0b0000_0000_0001,
    Entity    = 0b0000_0000_0010,
    Character = 0b0000_0000_0100,
    Creature  = 0b0000_0000_1000,
    Faction   = 0b0000_0001_0000,
    Location  = 0b0000_0010_0000,
    Item      = 0b0000_0100_0000,
    Artifact  = 0b0000_1000_0000,
    Spell     = 0b0001_0000_0000,
    Event     = 0b0010_0000_0000,
    Concept   = 0b0100_0000_0000,
    Document  = 0b1000_0000_0000,
}

/// A set of allowed entity kinds (bitflags)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KindSet(u16);

impl KindSet {
    pub const fn new() -> Self {
        KindSet(0)
    }
    
    pub const fn with(self, kind: Kind) -> Self {
        KindSet(self.0 | kind as u16)
    }
    
    pub const fn contains(&self, kind: Kind) -> bool {
        (self.0 & kind as u16) != 0 || (self.0 & Kind::Any as u16) != 0
    }
    
    pub fn contains_str(&self, kind_str: &str) -> bool {
        match kind_str.to_uppercase().as_str() {
            "CHARACTER" => self.contains(Kind::Character),
            "CREATURE" => self.contains(Kind::Creature),
            "FACTION" => self.contains(Kind::Faction),
            "LOCATION" => self.contains(Kind::Location),
            "ITEM" => self.contains(Kind::Item),
            "ARTIFACT" => self.contains(Kind::Artifact),
            "SPELL" => self.contains(Kind::Spell),
            "EVENT" => self.contains(Kind::Event),
            "CONCEPT" => self.contains(Kind::Concept),
            "DOCUMENT" => self.contains(Kind::Document),
            _ => self.contains(Kind::Any), // Unknown kinds pass if Any is set
        }
    }
}

// Convenience: Kind | Kind -> KindSet
impl std::ops::BitOr for Kind {
    type Output = KindSet;
    fn bitor(self, rhs: Self) -> KindSet {
        KindSet(self as u16 | rhs as u16)
    }
}

impl std::ops::BitOr<Kind> for KindSet {
    type Output = KindSet;
    fn bitor(self, rhs: Kind) -> KindSet {
        KindSet(self.0 | rhs as u16)
    }
}

// =============================================================================
// Relation Enum - All Relation Types
// =============================================================================

/// All supported relation types, including inverses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    // Combat
    Killed,
    KilledBy,
    Defeated,
    DefeatedBy,
    Wounded,
    WoundedBy,
    Attacked,
    AttackedBy,
    Fought,
    Captured,
    CapturedBy,
    
    // Social
    Loves,
    LovedBy,
    Hates,
    HatedBy,
    Married,
    ParentOf,
    ChildOf,
    SiblingOf,
    Betrayed,
    BetrayedBy,
    Befriended,
    Rescued,
    RescuedBy,
    
    // Hierarchy
    Leads,
    LedBy,
    Commands,
    CommandedBy,
    Rules,
    RuledBy,
    MemberOf,
    HasMember,
    Serves,
    ServedBy,
    Obeys,
    ObeyedBy,
    Founded,
    FoundedBy,
    
    // Knowledge
    Taught,
    TaughtBy,
    Mentored,
    MentoredBy,
    Trained,
    TrainedBy,
    LearnedFrom,
    KnowsSecretOf,
    Discovered,
    DiscoveredBy,
    
    // Possession
    Owns,
    OwnedBy,
    Possesses,
    PossessedBy,
    Wields,
    WieldedBy,
    Guards,
    GuardedBy,
    Stole,
    StolenBy,
    Gave,
    Received,
    Created,
    CreatedBy,
    Destroyed,
    DestroyedBy,
    
    // Location
    LocatedIn,
    Contains,
    PartOf,
    HasPart,
    LivesIn,
    TraveledTo,
    EnteredFrom,
    Left,
    FledFrom,
    OriginatedFrom,
    Borders,
    
    // Communication
    Told,
    ToldBy,
    Warned,
    WarnedBy,
    Promised,
    LiedTo,
    RevealedTo,
    
    // Magic
    Cast,
    Enchanted,
    EnchantedBy,
    Cursed,
    CursedBy,
    Blessed,
    BlessedBy,
    Summoned,
    SummonedBy,
    Banished,
    BanishedBy,
    Bound,
    BoundBy,
    Sealed,
    SealedBy,
    
    // State
    IsA,
    Became,
    Died,
    Resurrected,
    ResurrectedBy,
    Imprisoned,
    ImprisonedBy,
    Freed,
    FreedBy,
    
    // Temporal
    Precedes,
    Follows,
    Caused,
    CausedBy,
    Prevented,
    PreventedBy,
    Witnessed,
}

impl Relation {
    /// Convert relation enum to canonical string form
    pub fn as_str(&self) -> &'static str {
        match self {
            // Combat
            Relation::Killed => "KILLED",
            Relation::KilledBy => "KILLED_BY",
            Relation::Defeated => "DEFEATED",
            Relation::DefeatedBy => "DEFEATED_BY",
            Relation::Wounded => "WOUNDED",
            Relation::WoundedBy => "WOUNDED_BY",
            Relation::Attacked => "ATTACKED",
            Relation::AttackedBy => "ATTACKED_BY",
            Relation::Fought => "FOUGHT",
            Relation::Captured => "CAPTURED",
            Relation::CapturedBy => "CAPTURED_BY",
            
            // Social
            Relation::Loves => "LOVES",
            Relation::LovedBy => "LOVED_BY",
            Relation::Hates => "HATES",
            Relation::HatedBy => "HATED_BY",
            Relation::Married => "MARRIED",
            Relation::ParentOf => "PARENT_OF",
            Relation::ChildOf => "CHILD_OF",
            Relation::SiblingOf => "SIBLING_OF",
            Relation::Betrayed => "BETRAYED",
            Relation::BetrayedBy => "BETRAYED_BY",
            Relation::Befriended => "BEFRIENDED",
            Relation::Rescued => "RESCUED",
            Relation::RescuedBy => "RESCUED_BY",
            
            // Hierarchy
            Relation::Leads => "LEADS",
            Relation::LedBy => "LED_BY",
            Relation::Commands => "COMMANDS",
            Relation::CommandedBy => "COMMANDED_BY",
            Relation::Rules => "RULES",
            Relation::RuledBy => "RULED_BY",
            Relation::MemberOf => "MEMBER_OF",
            Relation::HasMember => "HAS_MEMBER",
            Relation::Serves => "SERVES",
            Relation::ServedBy => "SERVED_BY",
            Relation::Obeys => "OBEYS",
            Relation::ObeyedBy => "OBEYED_BY",
            Relation::Founded => "FOUNDED",
            Relation::FoundedBy => "FOUNDED_BY",
            
            // Knowledge
            Relation::Taught => "TAUGHT",
            Relation::TaughtBy => "TAUGHT_BY",
            Relation::Mentored => "MENTORED",
            Relation::MentoredBy => "MENTORED_BY",
            Relation::Trained => "TRAINED",
            Relation::TrainedBy => "TRAINED_BY",
            Relation::LearnedFrom => "LEARNED_FROM",
            Relation::KnowsSecretOf => "KNOWS_SECRET_OF",
            Relation::Discovered => "DISCOVERED",
            Relation::DiscoveredBy => "DISCOVERED_BY",
            
            // Possession
            Relation::Owns => "OWNS",
            Relation::OwnedBy => "OWNED_BY",
            Relation::Possesses => "POSSESSES",
            Relation::PossessedBy => "POSSESSED_BY",
            Relation::Wields => "WIELDS",
            Relation::WieldedBy => "WIELDED_BY",
            Relation::Guards => "GUARDS",
            Relation::GuardedBy => "GUARDED_BY",
            Relation::Stole => "STOLE",
            Relation::StolenBy => "STOLEN_BY",
            Relation::Gave => "GAVE",
            Relation::Received => "RECEIVED",
            Relation::Created => "CREATED",
            Relation::CreatedBy => "CREATED_BY",
            Relation::Destroyed => "DESTROYED",
            Relation::DestroyedBy => "DESTROYED_BY",
            
            // Location
            Relation::LocatedIn => "LOCATED_IN",
            Relation::Contains => "CONTAINS",
            Relation::PartOf => "PART_OF",
            Relation::HasPart => "HAS_PART",
            Relation::LivesIn => "LIVES_IN",
            Relation::TraveledTo => "TRAVELED_TO",
            Relation::EnteredFrom => "ENTERED_FROM",
            Relation::Left => "LEFT",
            Relation::FledFrom => "FLED_FROM",
            Relation::OriginatedFrom => "ORIGINATED_FROM",
            Relation::Borders => "BORDERS",
            
            // Communication
            Relation::Told => "TOLD",
            Relation::ToldBy => "TOLD_BY",
            Relation::Warned => "WARNED",
            Relation::WarnedBy => "WARNED_BY",
            Relation::Promised => "PROMISED",
            Relation::LiedTo => "LIED_TO",
            Relation::RevealedTo => "REVEALED_TO",
            
            // Magic
            Relation::Cast => "CAST",
            Relation::Enchanted => "ENCHANTED",
            Relation::EnchantedBy => "ENCHANTED_BY",
            Relation::Cursed => "CURSED",
            Relation::CursedBy => "CURSED_BY",
            Relation::Blessed => "BLESSED",
            Relation::BlessedBy => "BLESSED_BY",
            Relation::Summoned => "SUMMONED",
            Relation::SummonedBy => "SUMMONED_BY",
            Relation::Banished => "BANISHED",
            Relation::BanishedBy => "BANISHED_BY",
            Relation::Bound => "BOUND",
            Relation::BoundBy => "BOUND_BY",
            Relation::Sealed => "SEALED",
            Relation::SealedBy => "SEALED_BY",
            
            // State
            Relation::IsA => "IS_A",
            Relation::Became => "BECAME",
            Relation::Died => "DIED",
            Relation::Resurrected => "RESURRECTED",
            Relation::ResurrectedBy => "RESURRECTED_BY",
            Relation::Imprisoned => "IMPRISONED",
            Relation::ImprisonedBy => "IMPRISONED_BY",
            Relation::Freed => "FREED",
            Relation::FreedBy => "FREED_BY",
            
            // Temporal
            Relation::Precedes => "PRECEDES",
            Relation::Follows => "FOLLOWS",
            Relation::Caused => "CAUSED",
            Relation::CausedBy => "CAUSED_BY",
            Relation::Prevented => "PREVENTED",
            Relation::PreventedBy => "PREVENTED_BY",
            Relation::Witnessed => "WITNESSED",
        }
    }
    
    /// Try to parse a string into a Relation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "KILLED" => Some(Relation::Killed),
            "KILLED_BY" => Some(Relation::KilledBy),
            "DEFEATED" => Some(Relation::Defeated),
            "DEFEATED_BY" => Some(Relation::DefeatedBy),
            "WOUNDED" => Some(Relation::Wounded),
            "WOUNDED_BY" => Some(Relation::WoundedBy),
            "ATTACKED" => Some(Relation::Attacked),
            "ATTACKED_BY" => Some(Relation::AttackedBy),
            "FOUGHT" => Some(Relation::Fought),
            "CAPTURED" => Some(Relation::Captured),
            "CAPTURED_BY" => Some(Relation::CapturedBy),
            
            "LOVES" => Some(Relation::Loves),
            "LOVED_BY" => Some(Relation::LovedBy),
            "HATES" => Some(Relation::Hates),
            "HATED_BY" => Some(Relation::HatedBy),
            "MARRIED" => Some(Relation::Married),
            "PARENT_OF" => Some(Relation::ParentOf),
            "CHILD_OF" => Some(Relation::ChildOf),
            "SIBLING_OF" => Some(Relation::SiblingOf),
            "BETRAYED" => Some(Relation::Betrayed),
            "BETRAYED_BY" => Some(Relation::BetrayedBy),
            "BEFRIENDED" => Some(Relation::Befriended),
            "RESCUED" => Some(Relation::Rescued),
            "RESCUED_BY" => Some(Relation::RescuedBy),
            
            "LEADS" => Some(Relation::Leads),
            "LED_BY" => Some(Relation::LedBy),
            "COMMANDS" => Some(Relation::Commands),
            "COMMANDED_BY" => Some(Relation::CommandedBy),
            "RULES" => Some(Relation::Rules),
            "RULED_BY" => Some(Relation::RuledBy),
            "MEMBER_OF" => Some(Relation::MemberOf),
            "HAS_MEMBER" => Some(Relation::HasMember),
            "SERVES" => Some(Relation::Serves),
            "SERVED_BY" => Some(Relation::ServedBy),
            "OBEYS" => Some(Relation::Obeys),
            "OBEYED_BY" => Some(Relation::ObeyedBy),
            "FOUNDED" => Some(Relation::Founded),
            "FOUNDED_BY" => Some(Relation::FoundedBy),
            
            "TAUGHT" => Some(Relation::Taught),
            "TAUGHT_BY" => Some(Relation::TaughtBy),
            "MENTORED" => Some(Relation::Mentored),
            "MENTORED_BY" => Some(Relation::MentoredBy),
            "TRAINED" => Some(Relation::Trained),
            "TRAINED_BY" => Some(Relation::TrainedBy),
            "LEARNED_FROM" => Some(Relation::LearnedFrom),
            "KNOWS_SECRET_OF" => Some(Relation::KnowsSecretOf),
            "DISCOVERED" => Some(Relation::Discovered),
            "DISCOVERED_BY" => Some(Relation::DiscoveredBy),
            
            "OWNS" => Some(Relation::Owns),
            "OWNED_BY" => Some(Relation::OwnedBy),
            "POSSESSES" => Some(Relation::Possesses),
            "POSSESSED_BY" => Some(Relation::PossessedBy),
            "WIELDS" => Some(Relation::Wields),
            "WIELDED_BY" => Some(Relation::WieldedBy),
            "GUARDS" => Some(Relation::Guards),
            "GUARDED_BY" => Some(Relation::GuardedBy),
            "STOLE" => Some(Relation::Stole),
            "STOLEN_BY" => Some(Relation::StolenBy),
            "GAVE" => Some(Relation::Gave),
            "RECEIVED" => Some(Relation::Received),
            "CREATED" => Some(Relation::Created),
            "CREATED_BY" => Some(Relation::CreatedBy),
            "DESTROYED" => Some(Relation::Destroyed),
            "DESTROYED_BY" => Some(Relation::DestroyedBy),
            
            "LOCATED_IN" => Some(Relation::LocatedIn),
            "CONTAINS" => Some(Relation::Contains),
            "PART_OF" => Some(Relation::PartOf),
            "HAS_PART" => Some(Relation::HasPart),
            "LIVES_IN" => Some(Relation::LivesIn),
            "TRAVELED_TO" => Some(Relation::TraveledTo),
            "ENTERED_FROM" => Some(Relation::EnteredFrom),
            "LEFT" => Some(Relation::Left),
            "FLED_FROM" => Some(Relation::FledFrom),
            "ORIGINATED_FROM" => Some(Relation::OriginatedFrom),
            "BORDERS" => Some(Relation::Borders),
            
            "TOLD" => Some(Relation::Told),
            "TOLD_BY" => Some(Relation::ToldBy),
            "WARNED" => Some(Relation::Warned),
            "WARNED_BY" => Some(Relation::WarnedBy),
            "PROMISED" => Some(Relation::Promised),
            "LIED_TO" => Some(Relation::LiedTo),
            "REVEALED_TO" => Some(Relation::RevealedTo),
            
            "CAST" => Some(Relation::Cast),
            "ENCHANTED" => Some(Relation::Enchanted),
            "ENCHANTED_BY" => Some(Relation::EnchantedBy),
            "CURSED" => Some(Relation::Cursed),
            "CURSED_BY" => Some(Relation::CursedBy),
            "BLESSED" => Some(Relation::Blessed),
            "BLESSED_BY" => Some(Relation::BlessedBy),
            "SUMMONED" => Some(Relation::Summoned),
            "SUMMONED_BY" => Some(Relation::SummonedBy),
            "BANISHED" => Some(Relation::Banished),
            "BANISHED_BY" => Some(Relation::BanishedBy),
            "BOUND" => Some(Relation::Bound),
            "BOUND_BY" => Some(Relation::BoundBy),
            "SEALED" => Some(Relation::Sealed),
            "SEALED_BY" => Some(Relation::SealedBy),
            
            "IS_A" => Some(Relation::IsA),
            "BECAME" => Some(Relation::Became),
            "DIED" => Some(Relation::Died),
            "RESURRECTED" => Some(Relation::Resurrected),
            "RESURRECTED_BY" => Some(Relation::ResurrectedBy),
            "IMPRISONED" => Some(Relation::Imprisoned),
            "IMPRISONED_BY" => Some(Relation::ImprisonedBy),
            "FREED" => Some(Relation::Freed),
            "FREED_BY" => Some(Relation::FreedBy),
            
            "PRECEDES" => Some(Relation::Precedes),
            "FOLLOWS" => Some(Relation::Follows),
            "CAUSED" => Some(Relation::Caused),
            "CAUSED_BY" => Some(Relation::CausedBy),
            "PREVENTED" => Some(Relation::Prevented),
            "PREVENTED_BY" => Some(Relation::PreventedBy),
            "WITNESSED" => Some(Relation::Witnessed),
            
            _ => None,
        }
    }
}

// =============================================================================
// Slot Specification
// =============================================================================

/// Role of an argument in a relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Subject,  // Primary actor
    Object,   // Primary target
    Actor,    // Alternative to Subject for action verbs
    Target,   // Alternative to Object
    Where,    // Location argument
    Tool,     // Instrument used
    Result,   // Outcome/product
}

/// Specification for a single argument slot
#[derive(Debug, Clone, Copy)]
pub struct SlotSpec {
    pub role: Role,
    pub kinds: KindSet,
    pub optional: bool,
}

impl SlotSpec {
    pub const fn new(role: Role, kinds: KindSet) -> Self {
        Self { role, kinds, optional: false }
    }
    
    pub const fn optional(role: Role, kinds: KindSet) -> Self {
        Self { role, kinds, optional: true }
    }
}

// =============================================================================
// RelationSchema
// =============================================================================

/// Complete schema for a relation type
#[derive(Debug, Clone)]
pub struct RelationSchema {
    pub rel: Relation,
    pub slots: &'static [SlotSpec],
    pub inverse: Option<Relation>,
    pub symmetric: bool,
    pub transitivity: Option<f32>,
}

impl RelationSchema {
    pub fn arity(&self) -> usize {
        self.slots.len()
    }
    
    pub fn is_binary(&self) -> bool {
        self.slots.len() == 2
    }
    
    pub fn is_unary(&self) -> bool {
        self.slots.len() == 1
    }
}

// =============================================================================
// Schema Registry
// =============================================================================

/// Runtime registry for looking up schemas
#[derive(Debug)]
pub struct SchemaRegistry {
    by_relation: HashMap<Relation, &'static RelationSchema>,
    by_string: HashMap<&'static str, &'static RelationSchema>,
}

impl SchemaRegistry {
    pub fn new(schemas: &'static [RelationSchema]) -> Self {
        let mut by_relation = HashMap::new();
        let mut by_string = HashMap::new();
        
        for schema in schemas {
            by_relation.insert(schema.rel, schema);
            by_string.insert(schema.rel.as_str(), schema);
        }
        
        Self { by_relation, by_string }
    }
    
    pub fn get(&self, rel: Relation) -> Option<&'static RelationSchema> {
        self.by_relation.get(&rel).copied()
    }
    
    pub fn get_by_str(&self, rel_str: &str) -> Option<&'static RelationSchema> {
        self.by_string.get(rel_str.to_uppercase().as_str()).copied()
    }
    
    pub fn get_inverse(&self, rel: Relation) -> Option<Relation> {
        self.get(rel).and_then(|s| s.inverse)
    }
    
    pub fn get_transitivity(&self, rel: Relation) -> Option<f32> {
        self.get(rel).and_then(|s| s.transitivity)
    }
    
    pub fn is_symmetric(&self, rel: Relation) -> bool {
        self.get(rel).map(|s| s.symmetric).unwrap_or(false)
    }
}

// =============================================================================
// Schema Validation
// =============================================================================

/// Schema validation error
#[derive(Debug)]
pub enum SchemaError {
    MissingInverse(Relation),
    InverseDoesNotPointBack { rel: Relation, inv: Relation },
    ArityMismatch { rel: Relation, inv: Relation, a: usize, b: usize },
    SymmetricWithInverse(Relation),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::MissingInverse(r) => write!(f, "inverse relation not found: {:?}", r),
            SchemaError::InverseDoesNotPointBack { rel, inv } => 
                write!(f, "inverse mismatch: {:?} -> {:?} does not point back", rel, inv),
            SchemaError::ArityMismatch { rel, inv, a, b } => 
                write!(f, "arity mismatch: {:?} has {} slots but inverse {:?} has {}", rel, a, inv, b),
            SchemaError::SymmetricWithInverse(r) => 
                write!(f, "symmetric relation cannot also define inverse: {:?}", r),
        }
    }
}

impl std::error::Error for SchemaError {}

/// Validate all schemas for consistency
pub fn validate_schemas(all: &[RelationSchema]) -> Result<(), SchemaError> {
    let map: HashMap<Relation, &RelationSchema> = all.iter().map(|s| (s.rel, s)).collect();

    for s in all {
        // Symmetric relations cannot also have an inverse
        if s.symmetric && s.inverse.is_some() {
            return Err(SchemaError::SymmetricWithInverse(s.rel));
        }
        
        // If inverse is defined, it must exist and point back
        if let Some(inv) = s.inverse {
            let inv_s = map.get(&inv).ok_or(SchemaError::MissingInverse(inv))?;
            
            // Inverse must point back (unless it's symmetric)
            if inv_s.inverse != Some(s.rel) && !inv_s.symmetric {
                return Err(SchemaError::InverseDoesNotPointBack { rel: s.rel, inv });
            }
            
            // Arity must match
            if s.slots.len() != inv_s.slots.len() {
                return Err(SchemaError::ArityMismatch {
                    rel: s.rel,
                    inv,
                    a: s.slots.len(),
                    b: inv_s.slots.len(),
                });
            }
        }
    }
    Ok(())
}

// =============================================================================
// Schema Table - The Actual Schemas
// =============================================================================

/// Helper to create KindSet in const context
const fn kinds(k: Kind) -> KindSet {
    KindSet(k as u16)
}

const fn kinds2(a: Kind, b: Kind) -> KindSet {
    KindSet(a as u16 | b as u16)
}

const fn kinds3(a: Kind, b: Kind, c: Kind) -> KindSet {
    KindSet(a as u16 | b as u16 | c as u16)
}

/// All relation schemas
pub static RELATION_SCHEMAS: &[RelationSchema] = &[
    // =========================================================================
    // COMBAT
    // =========================================================================
    RelationSchema {
        rel: Relation::Killed,
        slots: &[
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Creature)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Creature)),
        ],
        inverse: Some(Relation::KilledBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::KilledBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Creature)),
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Creature)),
        ],
        inverse: Some(Relation::Killed),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Defeated,
        slots: &[
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: Some(Relation::DefeatedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::DefeatedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: Some(Relation::Defeated),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Fought,
        slots: &[
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: None,
        symmetric: true,
        transitivity: None,
    },
    
    // =========================================================================
    // SOCIAL
    // =========================================================================
    RelationSchema {
        rel: Relation::Loves,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::LovedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::LovedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Loves),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Married,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: None,
        symmetric: true,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::ParentOf,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::ChildOf),
        symmetric: false,
        transitivity: Some(0.95),
    },
    RelationSchema {
        rel: Relation::ChildOf,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::ParentOf),
        symmetric: false,
        transitivity: Some(0.95),
    },
    RelationSchema {
        rel: Relation::SiblingOf,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: None,
        symmetric: true,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Betrayed,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::BetrayedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::BetrayedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Betrayed),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Befriended,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: None,
        symmetric: true,
        transitivity: None,
    },
    
    // =========================================================================
    // HIERARCHY
    // =========================================================================
    RelationSchema {
        rel: Relation::Leads,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Faction)),
        ],
        inverse: Some(Relation::LedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::LedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Faction)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Leads),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Commands,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: Some(Relation::CommandedBy),
        symmetric: false,
        transitivity: Some(0.9),
    },
    RelationSchema {
        rel: Relation::CommandedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Commands),
        symmetric: false,
        transitivity: Some(0.9),
    },
    RelationSchema {
        rel: Relation::MemberOf,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Faction)),
        ],
        inverse: Some(Relation::HasMember),
        symmetric: false,
        transitivity: Some(0.95),
    },
    RelationSchema {
        rel: Relation::HasMember,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Faction)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::MemberOf),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Serves,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: Some(Relation::ServedBy),
        symmetric: false,
        transitivity: Some(0.8),
    },
    RelationSchema {
        rel: Relation::ServedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Serves),
        symmetric: false,
        transitivity: None,
    },
    
    // =========================================================================
    // KNOWLEDGE
    // =========================================================================
    RelationSchema {
        rel: Relation::Taught,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::TaughtBy),
        symmetric: false,
        transitivity: Some(0.7),
    },
    RelationSchema {
        rel: Relation::TaughtBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Taught),
        symmetric: false,
        transitivity: Some(0.7),
    },
    RelationSchema {
        rel: Relation::Mentored,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::MentoredBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::MentoredBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Mentored),
        symmetric: false,
        transitivity: None,
    },
    
    // =========================================================================
    // POSSESSION
    // =========================================================================
    RelationSchema {
        rel: Relation::Owns,
        slots: &[
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Faction)),
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Location)),
        ],
        inverse: Some(Relation::OwnedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::OwnedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Location)),
            SlotSpec::new(Role::Subject, kinds2(Kind::Character, Kind::Faction)),
        ],
        inverse: Some(Relation::Owns),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Guards,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds3(Kind::Item, Kind::Location, Kind::Character)),
        ],
        inverse: Some(Relation::GuardedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::GuardedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds3(Kind::Item, Kind::Location, Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Guards),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Created,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Artifact)),
        ],
        inverse: Some(Relation::CreatedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::CreatedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Artifact)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Created),
        symmetric: false,
        transitivity: None,
    },
    
    // =========================================================================
    // LOCATION
    // =========================================================================
    RelationSchema {
        rel: Relation::LocatedIn,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Any)),
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
        ],
        inverse: Some(Relation::Contains),
        symmetric: false,
        transitivity: Some(0.95),
    },
    RelationSchema {
        rel: Relation::Contains,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
            SlotSpec::new(Role::Subject, kinds(Kind::Any)),
        ],
        inverse: Some(Relation::LocatedIn),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::PartOf,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Location)),
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
        ],
        inverse: Some(Relation::HasPart),
        symmetric: false,
        transitivity: Some(0.99),
    },
    RelationSchema {
        rel: Relation::HasPart,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
            SlotSpec::new(Role::Subject, kinds(Kind::Location)),
        ],
        inverse: Some(Relation::PartOf),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::LivesIn,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Borders,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Location)),
            SlotSpec::new(Role::Object, kinds(Kind::Location)),
        ],
        inverse: None,
        symmetric: true,
        transitivity: None,
    },
    
    // =========================================================================
    // MAGIC
    // =========================================================================
    RelationSchema {
        rel: Relation::Cast,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Spell)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Enchanted,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Character)),
        ],
        inverse: Some(Relation::EnchantedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::EnchantedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Item, Kind::Character)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Enchanted),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Cursed,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Item)),
        ],
        inverse: Some(Relation::CursedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::CursedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Character, Kind::Item)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Cursed),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Summoned,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds2(Kind::Creature, Kind::Entity)),
        ],
        inverse: Some(Relation::SummonedBy),
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::SummonedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds2(Kind::Creature, Kind::Entity)),
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: Some(Relation::Summoned),
        symmetric: false,
        transitivity: None,
    },
    
    // =========================================================================
    // STATE
    // =========================================================================
    RelationSchema {
        rel: Relation::IsA,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Any)),
            SlotSpec::new(Role::Object, kinds(Kind::Concept)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: Some(1.0),
    },
    RelationSchema {
        rel: Relation::Became,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Concept)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: None,
    },
    RelationSchema {
        rel: Relation::Died,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: None,
    },
    
    // =========================================================================
    // TEMPORAL
    // =========================================================================
    RelationSchema {
        rel: Relation::Precedes,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Event)),
            SlotSpec::new(Role::Object, kinds(Kind::Event)),
        ],
        inverse: Some(Relation::Follows),
        symmetric: false,
        transitivity: Some(1.0),
    },
    RelationSchema {
        rel: Relation::Follows,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Event)),
            SlotSpec::new(Role::Subject, kinds(Kind::Event)),
        ],
        inverse: Some(Relation::Precedes),
        symmetric: false,
        transitivity: Some(1.0),
    },
    RelationSchema {
        rel: Relation::Caused,
        slots: &[
            SlotSpec::new(Role::Subject, kinds2(Kind::Event, Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Event)),
        ],
        inverse: Some(Relation::CausedBy),
        symmetric: false,
        transitivity: Some(0.7),
    },
    RelationSchema {
        rel: Relation::CausedBy,
        slots: &[
            SlotSpec::new(Role::Object, kinds(Kind::Event)),
            SlotSpec::new(Role::Subject, kinds2(Kind::Event, Kind::Character)),
        ],
        inverse: Some(Relation::Caused),
        symmetric: false,
        transitivity: Some(0.7),
    },
    RelationSchema {
        rel: Relation::Witnessed,
        slots: &[
            SlotSpec::new(Role::Subject, kinds(Kind::Character)),
            SlotSpec::new(Role::Object, kinds(Kind::Event)),
        ],
        inverse: None,
        symmetric: false,
        transitivity: None,
    },
];

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation() {
        // Should not panic
        validate_schemas(RELATION_SCHEMAS).expect("Schema validation failed");
    }

    #[test]
    fn test_kind_set_contains() {
        let set = Kind::Character | Kind::Creature;
        assert!(set.contains(Kind::Character));
        assert!(set.contains(Kind::Creature));
        assert!(!set.contains(Kind::Location));
    }

    #[test]
    fn test_kind_set_any() {
        let set = KindSet::new().with(Kind::Any);
        assert!(set.contains(Kind::Character));
        assert!(set.contains(Kind::Location));
        assert!(set.contains(Kind::Item));
    }

    #[test]
    fn test_relation_as_str() {
        assert_eq!(Relation::Killed.as_str(), "KILLED");
        assert_eq!(Relation::KilledBy.as_str(), "KILLED_BY");
        assert_eq!(Relation::MemberOf.as_str(), "MEMBER_OF");
    }

    #[test]
    fn test_relation_from_str() {
        assert_eq!(Relation::from_str("KILLED"), Some(Relation::Killed));
        assert_eq!(Relation::from_str("killed"), Some(Relation::Killed));
        assert_eq!(Relation::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_schema_registry() {
        let registry = SchemaRegistry::new(RELATION_SCHEMAS);
        
        let killed_schema = registry.get(Relation::Killed).unwrap();
        assert_eq!(killed_schema.inverse, Some(Relation::KilledBy));
        assert!(killed_schema.is_binary());
        
        let died_schema = registry.get(Relation::Died).unwrap();
        assert!(died_schema.is_unary());
    }

    #[test]
    fn test_transitivity_lookup() {
        let registry = SchemaRegistry::new(RELATION_SCHEMAS);
        
        assert_eq!(registry.get_transitivity(Relation::ParentOf), Some(0.95));
        assert_eq!(registry.get_transitivity(Relation::Killed), None);
        assert_eq!(registry.get_transitivity(Relation::LocatedIn), Some(0.95));
    }

    #[test]
    fn test_symmetric_relations() {
        let registry = SchemaRegistry::new(RELATION_SCHEMAS);
        
        assert!(registry.is_symmetric(Relation::Married));
        assert!(registry.is_symmetric(Relation::Fought));
        assert!(registry.is_symmetric(Relation::Borders));
        assert!(!registry.is_symmetric(Relation::Killed));
    }

    #[test]
    fn test_kind_set_contains_str() {
        let set = Kind::Character | Kind::Creature;
        assert!(set.contains_str("CHARACTER"));
        assert!(set.contains_str("character"));
        assert!(set.contains_str("CREATURE"));
        assert!(!set.contains_str("LOCATION"));
    }

    #[test]
    fn test_schema_count() {
        // Ensure we have a good number of schemas
        assert!(RELATION_SCHEMAS.len() >= 40, "Expected at least 40 schemas");
    }
}
