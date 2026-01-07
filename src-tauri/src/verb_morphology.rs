//! VerbLexicon - Unified Verb Morphology + Semantics (Native Tauri)
//!
//! A cohesive system that handles:
//! - Inflection patterns (kill → kills, killed, killing)
//! - Semantic relation mapping (kill → KILLED)
//! - Domain categorization (Combat, Social, etc.)
//! - Transitivity for confidence scoring
//!
//! ~150+ base verbs → 600+ recognized forms

use std::collections::HashMap;

// =============================================================================
// Core Types
// =============================================================================

/// Conjugation pattern for English verbs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbPattern {
    /// Regular: walk → walks, walked, walking
    Regular,
    /// -e ending: love → loves, loved, loving (drop e for -ing)
    RegularE,
    /// Consonant doubling: stop → stops, stopped, stopping
    DoubleConsonant,
    /// -y to -ies/-ied: carry → carries, carried, carrying
    YToI,
    /// Irregular with explicit past and past participle
    Irregular {
        past: &'static str,
        past_participle: &'static str,
    },
}

/// Semantic domain for verb categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbDomain {
    Combat,
    Magic,
    Social,
    Leadership,
    Teaching,
    Possession,
    Protection,
    Creation,
    Movement,
    Communication,
    Cognition,
    Emotion,
    StateChange,
}

/// Verb transitivity for argument structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transitivity {
    /// Requires direct object: "X killed Y"
    Transitive,
    /// No direct object: "X slept"
    Intransitive,
    /// Can be either: "X ate" or "X ate Y"
    Ambitransitive,
    /// Takes two objects: "X gave Y to Z"
    Ditransitive,
}

/// A verb entry with morphology AND semantics
#[derive(Debug, Clone)]
pub struct VerbEntry {
    pub base: &'static str,
    pub pattern: VerbPattern,
    pub relation: &'static str,
    pub domain: VerbDomain,
    pub transitivity: Transitivity,
}

impl VerbEntry {
    pub const fn regular(base: &'static str, relation: &'static str, domain: VerbDomain) -> Self {
        Self {
            base,
            pattern: VerbPattern::Regular,
            relation,
            domain,
            transitivity: Transitivity::Transitive,
        }
    }

    pub const fn e_ending(base: &'static str, relation: &'static str, domain: VerbDomain) -> Self {
        Self {
            base,
            pattern: VerbPattern::RegularE,
            relation,
            domain,
            transitivity: Transitivity::Transitive,
        }
    }

    pub const fn double_consonant(base: &'static str, relation: &'static str, domain: VerbDomain) -> Self {
        Self {
            base,
            pattern: VerbPattern::DoubleConsonant,
            relation,
            domain,
            transitivity: Transitivity::Transitive,
        }
    }

    pub const fn y_to_i(base: &'static str, relation: &'static str, domain: VerbDomain) -> Self {
        Self {
            base,
            pattern: VerbPattern::YToI,
            relation,
            domain,
            transitivity: Transitivity::Transitive,
        }
    }

    pub const fn irregular(
        base: &'static str,
        past: &'static str,
        past_participle: &'static str,
        relation: &'static str,
        domain: VerbDomain,
    ) -> Self {
        Self {
            base,
            pattern: VerbPattern::Irregular { past, past_participle },
            relation,
            domain,
            transitivity: Transitivity::Transitive,
        }
    }

    /// Generate all inflected forms of this verb
    pub fn inflections(&self) -> Vec<String> {
        let base = self.base;
        
        match &self.pattern {
            VerbPattern::Regular => {
                let third_singular = Self::third_person_singular(base);
                vec![
                    base.to_string(),
                    third_singular,
                    format!("{}ed", base),
                    format!("{}ing", base),
                ]
            },
            
            VerbPattern::RegularE => {
                let stem = &base[..base.len() - 1];
                vec![
                    base.to_string(),
                    format!("{}s", base),
                    format!("{}d", base),
                    format!("{}ing", stem),
                ]
            },
            
            VerbPattern::DoubleConsonant => {
                let last_char = base.chars().last().unwrap();
                vec![
                    base.to_string(),
                    format!("{}s", base),
                    format!("{}{}ed", base, last_char),
                    format!("{}{}ing", base, last_char),
                ]
            },
            
            VerbPattern::YToI => {
                let stem = &base[..base.len() - 1];
                vec![
                    base.to_string(),
                    format!("{}ies", stem),
                    format!("{}ied", stem),
                    format!("{}ying", stem),
                ]
            },
            
            VerbPattern::Irregular { past, past_participle } => {
                let third_singular = Self::third_person_singular(base);
                let mut forms = vec![
                    base.to_string(),
                    third_singular,
                    past.to_string(),
                    format!("{}ing", base),
                ];
                if past != past_participle {
                    forms.push(past_participle.to_string());
                }
                forms
            },
        }
    }
    
    fn third_person_singular(base: &str) -> String {
        if base.ends_with('s') || base.ends_with('x') 
            || base.ends_with('z') || base.ends_with("sh") || base.ends_with("ch") {
            format!("{}es", base)
        } else if base.ends_with('y') && !base.ends_with("ay") && !base.ends_with("ey") 
            && !base.ends_with("oy") && !base.ends_with("uy") {
            format!("{}ies", &base[..base.len()-1])
        } else {
            format!("{}s", base)
        }
    }
}

// =============================================================================
// Verb Table (Comprehensive Coverage)
// =============================================================================

const VERB_TABLE: &[VerbEntry] = &[
    // COMBAT / CONFLICT
    VerbEntry::regular("kill", "KILLED", VerbDomain::Combat),
    VerbEntry::irregular("slay", "slew", "slain", "KILLED", VerbDomain::Combat),
    VerbEntry::regular("murder", "KILLED", VerbDomain::Combat),
    VerbEntry::e_ending("assassinate", "KILLED", VerbDomain::Combat),
    VerbEntry::e_ending("execute", "KILLED", VerbDomain::Combat),
    VerbEntry::regular("slaughter", "KILLED", VerbDomain::Combat),
    VerbEntry::regular("defeat", "DEFEATED", VerbDomain::Combat),
    VerbEntry::regular("vanquish", "DEFEATED", VerbDomain::Combat),
    VerbEntry::regular("conquer", "DEFEATED", VerbDomain::Combat),
    VerbEntry::e_ending("overcome", "DEFEATED", VerbDomain::Combat),
    VerbEntry::regular("best", "DEFEATED", VerbDomain::Combat),
    VerbEntry::e_ending("subdue", "DEFEATED", VerbDomain::Combat),
    VerbEntry::regular("crush", "DEFEATED", VerbDomain::Combat),
    VerbEntry::irregular("fight", "fought", "fought", "FOUGHT", VerbDomain::Combat),
    VerbEntry::e_ending("battle", "FOUGHT", VerbDomain::Combat),
    VerbEntry::regular("duel", "FOUGHT", VerbDomain::Combat),
    VerbEntry::regular("clash", "FOUGHT", VerbDomain::Combat),
    VerbEntry::regular("combat", "FOUGHT", VerbDomain::Combat),
    VerbEntry::regular("attack", "ATTACKED", VerbDomain::Combat),
    VerbEntry::regular("assault", "ATTACKED", VerbDomain::Combat),
    VerbEntry::regular("ambush", "ATTACKED", VerbDomain::Combat),
    VerbEntry::irregular("strike", "struck", "struck", "ATTACKED", VerbDomain::Combat),
    VerbEntry::regular("raid", "ATTACKED", VerbDomain::Combat),
    VerbEntry::e_ending("besiege", "ATTACKED", VerbDomain::Combat),
    VerbEntry::e_ending("invade", "ATTACKED", VerbDomain::Combat),
    VerbEntry::regular("wound", "WOUNDED", VerbDomain::Combat),
    VerbEntry::e_ending("injure", "WOUNDED", VerbDomain::Combat),
    VerbEntry::irregular("hurt", "hurt", "hurt", "WOUNDED", VerbDomain::Combat),
    VerbEntry::regular("maim", "WOUNDED", VerbDomain::Combat),
    
    // MAGIC / FANTASY
    VerbEntry::irregular("cast", "cast", "cast", "CAST", VerbDomain::Magic),
    VerbEntry::regular("enchant", "ENCHANTED", VerbDomain::Magic),
    VerbEntry::e_ending("curse", "CURSED", VerbDomain::Magic),
    VerbEntry::regular("hex", "CURSED", VerbDomain::Magic),
    VerbEntry::regular("bless", "BLESSED", VerbDomain::Magic),
    VerbEntry::regular("summon", "SUMMONED", VerbDomain::Magic),
    VerbEntry::regular("banish", "BANISHED", VerbDomain::Magic),
    VerbEntry::e_ending("conjure", "CONJURED", VerbDomain::Magic),
    VerbEntry::regular("bewitch", "ENCHANTED", VerbDomain::Magic),
    VerbEntry::regular("charm", "ENCHANTED", VerbDomain::Magic),
    VerbEntry::regular("dispel", "DISPELLED", VerbDomain::Magic),
    VerbEntry::regular("transform", "TRANSFORMED", VerbDomain::Magic),
    VerbEntry::e_ending("transmute", "TRANSFORMED", VerbDomain::Magic),
    VerbEntry::regular("heal", "HEALED", VerbDomain::Magic),
    VerbEntry::e_ending("resurrect", "RESURRECTED", VerbDomain::Magic),
    VerbEntry::regular("seal", "SEALED", VerbDomain::Magic),
    VerbEntry::regular("bind", "BOUND", VerbDomain::Magic),
    
    // SOCIAL / RELATIONSHIPS
    VerbEntry::e_ending("love", "LOVES", VerbDomain::Social),
    VerbEntry::e_ending("hate", "HATES", VerbDomain::Social),
    VerbEntry::y_to_i("marry", "MARRIED", VerbDomain::Social),
    VerbEntry::e_ending("divorce", "DIVORCED", VerbDomain::Social),
    VerbEntry::regular("befriend", "BEFRIENDED", VerbDomain::Social),
    VerbEntry::regular("betray", "BETRAYED", VerbDomain::Social),
    VerbEntry::regular("trust", "TRUSTS", VerbDomain::Social),
    VerbEntry::regular("distrust", "DISTRUSTS", VerbDomain::Social),
    VerbEntry::y_to_i("ally", "ALLIED_WITH", VerbDomain::Social),
    VerbEntry::regular("rival", "RIVALS", VerbDomain::Social),
    VerbEntry::e_ending("admire", "ADMIRES", VerbDomain::Social),
    VerbEntry::e_ending("despise", "DESPISES", VerbDomain::Social),
    VerbEntry::e_ending("forgive", "FORGAVE", VerbDomain::Social),
    VerbEntry::regular("resent", "RESENTS", VerbDomain::Social),
    VerbEntry::irregular("meet", "met", "met", "MET", VerbDomain::Social),
    VerbEntry::regular("join", "JOINED", VerbDomain::Social),
    VerbEntry::regular("abandon", "ABANDONED", VerbDomain::Social),
    VerbEntry::e_ending("rescue", "RESCUED", VerbDomain::Social),
    VerbEntry::e_ending("save", "SAVED", VerbDomain::Social),
    VerbEntry::regular("help", "HELPED", VerbDomain::Social),
    
    // LEADERSHIP / AUTHORITY
    VerbEntry::irregular("lead", "led", "led", "LEADS", VerbDomain::Leadership),
    VerbEntry::regular("command", "COMMANDS", VerbDomain::Leadership),
    VerbEntry::e_ending("rule", "RULES", VerbDomain::Leadership),
    VerbEntry::regular("govern", "GOVERNS", VerbDomain::Leadership),
    VerbEntry::regular("reign", "RULES", VerbDomain::Leadership),
    VerbEntry::regular("control", "CONTROLS", VerbDomain::Leadership),
    VerbEntry::e_ending("dominate", "DOMINATES", VerbDomain::Leadership),
    VerbEntry::regular("obey", "OBEYS", VerbDomain::Leadership),
    VerbEntry::e_ending("serve", "SERVES", VerbDomain::Leadership),
    VerbEntry::regular("follow", "FOLLOWS", VerbDomain::Leadership),
    VerbEntry::regular("submit", "SUBMITS", VerbDomain::Leadership),
    VerbEntry::regular("rebel", "REBELLED", VerbDomain::Leadership),
    VerbEntry::e_ending("overthrow", "OVERTHREW", VerbDomain::Leadership),
    
    // TEACHING / MENTORSHIP
    VerbEntry::irregular("teach", "taught", "taught", "TAUGHT", VerbDomain::Teaching),
    VerbEntry::regular("mentor", "MENTORED", VerbDomain::Teaching),
    VerbEntry::regular("train", "TRAINED", VerbDomain::Teaching),
    VerbEntry::regular("tutor", "TUTORED", VerbDomain::Teaching),
    VerbEntry::e_ending("guide", "GUIDED", VerbDomain::Teaching),
    VerbEntry::regular("instruct", "INSTRUCTED", VerbDomain::Teaching),
    VerbEntry::e_ending("educate", "EDUCATED", VerbDomain::Teaching),
    VerbEntry::regular("coach", "COACHED", VerbDomain::Teaching),
    VerbEntry::irregular("learn", "learned", "learned", "LEARNED_FROM", VerbDomain::Teaching),
    VerbEntry::y_to_i("study", "STUDIED", VerbDomain::Teaching),
    
    // POSSESSION / OWNERSHIP
    VerbEntry::regular("own", "OWNS", VerbDomain::Possession),
    VerbEntry::regular("possess", "POSSESSES", VerbDomain::Possession),
    VerbEntry::irregular("hold", "held", "held", "HOLDS", VerbDomain::Possession),
    VerbEntry::irregular("keep", "kept", "kept", "KEEPS", VerbDomain::Possession),
    VerbEntry::irregular("steal", "stole", "stolen", "STOLE", VerbDomain::Possession),
    VerbEntry::regular("rob", "ROBBED", VerbDomain::Possession),
    VerbEntry::irregular("take", "took", "taken", "TOOK", VerbDomain::Possession),
    VerbEntry::irregular("give", "gave", "given", "GAVE", VerbDomain::Possession),
    VerbEntry::e_ending("receive", "RECEIVED", VerbDomain::Possession),
    VerbEntry::regular("inherit", "INHERITED", VerbDomain::Possession),
    VerbEntry::irregular("lose", "lost", "lost", "LOST", VerbDomain::Possession),
    VerbEntry::irregular("find", "found", "found", "FOUND", VerbDomain::Possession),
    VerbEntry::irregular("get", "got", "gotten", "GOT", VerbDomain::Possession),
    VerbEntry::e_ending("acquire", "ACQUIRED", VerbDomain::Possession),
    VerbEntry::regular("claim", "CLAIMED", VerbDomain::Possession),
    VerbEntry::regular("seize", "SEIZED", VerbDomain::Possession),
    
    // PROTECTION / DEFENSE
    VerbEntry::regular("guard", "GUARDS", VerbDomain::Protection),
    VerbEntry::regular("protect", "PROTECTS", VerbDomain::Protection),
    VerbEntry::regular("defend", "DEFENDS", VerbDomain::Protection),
    VerbEntry::regular("shield", "SHIELDS", VerbDomain::Protection),
    VerbEntry::regular("watch", "WATCHES", VerbDomain::Protection),
    VerbEntry::regular("ward", "WARDS", VerbDomain::Protection),
    VerbEntry::regular("shelter", "SHELTERS", VerbDomain::Protection),
    VerbEntry::e_ending("secure", "SECURED", VerbDomain::Protection),
    
    // CREATION / DESTRUCTION
    VerbEntry::e_ending("create", "CREATED", VerbDomain::Creation),
    VerbEntry::irregular("make", "made", "made", "MADE", VerbDomain::Creation),
    VerbEntry::irregular("build", "built", "built", "BUILT", VerbDomain::Creation),
    VerbEntry::e_ending("forge", "FORGED", VerbDomain::Creation),
    VerbEntry::regular("craft", "CRAFTED", VerbDomain::Creation),
    VerbEntry::regular("construct", "CONSTRUCTED", VerbDomain::Creation),
    VerbEntry::regular("invent", "INVENTED", VerbDomain::Creation),
    VerbEntry::e_ending("compose", "COMPOSED", VerbDomain::Creation),
    VerbEntry::irregular("write", "wrote", "written", "WROTE", VerbDomain::Creation),
    VerbEntry::irregular("draw", "drew", "drawn", "DREW", VerbDomain::Creation),
    VerbEntry::regular("destroy", "DESTROYED", VerbDomain::Creation),
    VerbEntry::regular("demolish", "DEMOLISHED", VerbDomain::Creation),
    VerbEntry::regular("ruin", "RUINED", VerbDomain::Creation),
    VerbEntry::irregular("break", "broke", "broken", "BROKE", VerbDomain::Creation),
    VerbEntry::irregular("tear", "tore", "torn", "TORE", VerbDomain::Creation),
    
    // MOVEMENT / LOCATION
    VerbEntry::regular("travel", "TRAVELED_TO", VerbDomain::Movement),
    VerbEntry::regular("journey", "TRAVELED_TO", VerbDomain::Movement),
    VerbEntry::regular("visit", "VISITED", VerbDomain::Movement),
    VerbEntry::regular("enter", "ENTERED", VerbDomain::Movement),
    VerbEntry::irregular("leave", "left", "left", "LEFT", VerbDomain::Movement),
    VerbEntry::irregular("flee", "fled", "fled", "FLED", VerbDomain::Movement),
    VerbEntry::e_ending("escape", "ESCAPED", VerbDomain::Movement),
    VerbEntry::e_ending("live", "LIVES_IN", VerbDomain::Movement),
    VerbEntry::regular("dwell", "DWELLS_IN", VerbDomain::Movement),
    VerbEntry::e_ending("reside", "RESIDES_IN", VerbDomain::Movement),
    VerbEntry::irregular("go", "went", "gone", "WENT_TO", VerbDomain::Movement),
    VerbEntry::irregular("come", "came", "come", "CAME_TO", VerbDomain::Movement),
    VerbEntry::irregular("run", "ran", "run", "RAN", VerbDomain::Movement),
    VerbEntry::regular("walk", "WALKED", VerbDomain::Movement),
    VerbEntry::irregular("fly", "flew", "flown", "FLEW", VerbDomain::Movement),
    VerbEntry::e_ending("arrive", "ARRIVED_AT", VerbDomain::Movement),
    VerbEntry::regular("depart", "DEPARTED", VerbDomain::Movement),
    VerbEntry::regular("return", "RETURNED", VerbDomain::Movement),
    VerbEntry::e_ending("pursue", "PURSUED", VerbDomain::Movement),
    VerbEntry::e_ending("chase", "CHASED", VerbDomain::Movement),
    
    // COMMUNICATION
    VerbEntry::irregular("say", "said", "said", "SAID_TO", VerbDomain::Communication),
    VerbEntry::irregular("tell", "told", "told", "TOLD", VerbDomain::Communication),
    VerbEntry::irregular("speak", "spoke", "spoken", "SPOKE_TO", VerbDomain::Communication),
    VerbEntry::regular("talk", "TALKED_TO", VerbDomain::Communication),
    VerbEntry::regular("ask", "ASKED", VerbDomain::Communication),
    VerbEntry::regular("answer", "ANSWERED", VerbDomain::Communication),
    VerbEntry::regular("call", "CALLED", VerbDomain::Communication),
    VerbEntry::regular("shout", "SHOUTED", VerbDomain::Communication),
    VerbEntry::regular("yell", "YELLED", VerbDomain::Communication),
    VerbEntry::regular("whisper", "WHISPERED", VerbDomain::Communication),
    VerbEntry::e_ending("declare", "DECLARED", VerbDomain::Communication),
    VerbEntry::e_ending("announce", "ANNOUNCED", VerbDomain::Communication),
    VerbEntry::regular("warn", "WARNED", VerbDomain::Communication),
    VerbEntry::regular("threaten", "THREATENED", VerbDomain::Communication),
    VerbEntry::e_ending("promise", "PROMISED", VerbDomain::Communication),
    
    // COGNITION / PERCEPTION
    VerbEntry::irregular("know", "knew", "known", "KNOWS", VerbDomain::Cognition),
    VerbEntry::irregular("think", "thought", "thought", "THINKS", VerbDomain::Cognition),
    VerbEntry::irregular("see", "saw", "seen", "SAW", VerbDomain::Cognition),
    VerbEntry::irregular("hear", "heard", "heard", "HEARD", VerbDomain::Cognition),
    VerbEntry::irregular("understand", "understood", "understood", "UNDERSTANDS", VerbDomain::Cognition),
    VerbEntry::e_ending("believe", "BELIEVES", VerbDomain::Cognition),
    VerbEntry::regular("remember", "REMEMBERS", VerbDomain::Cognition),
    VerbEntry::irregular("forget", "forgot", "forgotten", "FORGOT", VerbDomain::Cognition),
    VerbEntry::e_ending("realize", "REALIZED", VerbDomain::Cognition),
    VerbEntry::regular("discover", "DISCOVERED", VerbDomain::Cognition),
    VerbEntry::e_ending("recognize", "RECOGNIZED", VerbDomain::Cognition),
    VerbEntry::regular("suspect", "SUSPECTS", VerbDomain::Cognition),
    VerbEntry::regular("doubt", "DOUBTS", VerbDomain::Cognition),
    
    // EMOTION / DESIRE
    VerbEntry::regular("want", "WANTS", VerbDomain::Emotion),
    VerbEntry::regular("need", "NEEDS", VerbDomain::Emotion),
    VerbEntry::e_ending("desire", "DESIRES", VerbDomain::Emotion),
    VerbEntry::regular("fear", "FEARS", VerbDomain::Emotion),
    VerbEntry::regular("enjoy", "ENJOYS", VerbDomain::Emotion),
    VerbEntry::regular("wish", "WISHES", VerbDomain::Emotion),
    VerbEntry::e_ending("hope", "HOPES", VerbDomain::Emotion),
    VerbEntry::regular("dread", "DREADS", VerbDomain::Emotion),
    VerbEntry::regular("mourn", "MOURNS", VerbDomain::Emotion),
    VerbEntry::regular("grieve", "GRIEVES", VerbDomain::Emotion),
    
    // STATE CHANGE
    VerbEntry::irregular("be", "was", "been", "IS", VerbDomain::StateChange),
    VerbEntry::e_ending("become", "BECAME", VerbDomain::StateChange),
    VerbEntry::regular("remain", "REMAINS", VerbDomain::StateChange),
    VerbEntry::regular("exist", "EXISTS", VerbDomain::StateChange),
    VerbEntry::irregular("die", "died", "died", "DIED", VerbDomain::StateChange),
    VerbEntry::irregular("grow", "grew", "grown", "GREW", VerbDomain::StateChange),
    VerbEntry::e_ending("change", "CHANGED", VerbDomain::StateChange),
    VerbEntry::irregular("fall", "fell", "fallen", "FELL", VerbDomain::StateChange),
    VerbEntry::irregular("rise", "rose", "risen", "ROSE", VerbDomain::StateChange),
    VerbEntry::irregular("awake", "awoke", "awoken", "AWOKE", VerbDomain::StateChange),
];

// =============================================================================
// VerbLexicon
// =============================================================================

/// Unified verb lexicon with morphology + semantics
#[derive(Debug, Clone)]
pub struct VerbLexicon {
    /// form → (canonical relation, domain, transitivity)
    lookup: HashMap<String, (&'static str, VerbDomain, Transitivity)>,
}

impl Default for VerbLexicon {
    fn default() -> Self {
        Self::new()
    }
}

impl VerbLexicon {
    /// Create a new VerbLexicon from the built-in verb table
    pub fn new() -> Self {
        let mut lookup = HashMap::new();
        
        for entry in VERB_TABLE {
            let data = (entry.relation, entry.domain, entry.transitivity);
            for form in entry.inflections() {
                lookup.insert(form.to_lowercase(), data);
            }
        }
        
        // Add special cases
        Self::add_special_cases(&mut lookup);
        
        Self { lookup }
    }
    
    fn add_special_cases(lookup: &mut HashMap<String, (&'static str, VerbDomain, Transitivity)>) {
        // "have" is highly irregular
        let have_data = ("HAS", VerbDomain::Possession, Transitivity::Transitive);
        for form in ["have", "has", "had", "having"] {
            lookup.insert(form.to_string(), have_data);
        }
        
        // "be" conjugations
        let be_data = ("IS", VerbDomain::StateChange, Transitivity::Intransitive);
        for form in ["be", "am", "is", "are", "was", "were", "been", "being"] {
            lookup.insert(form.to_string(), be_data);
        }
        
        // "do" conjugations
        let do_data = ("DID", VerbDomain::StateChange, Transitivity::Transitive);
        for form in ["do", "does", "did", "done", "doing"] {
            lookup.insert(form.to_string(), do_data);
        }
    }
    
    /// Check if a word is a recognized verb
    pub fn is_verb(&self, word: &str) -> bool {
        self.lookup.contains_key(&word.to_lowercase())
    }
    
    /// Get the canonical relation type for a verb form
    pub fn get_relation(&self, word: &str) -> Option<&'static str> {
        self.lookup.get(&word.to_lowercase()).map(|(rel, _, _)| *rel)
    }
    
    /// Get the domain for a verb form
    pub fn get_domain(&self, word: &str) -> Option<VerbDomain> {
        self.lookup.get(&word.to_lowercase()).map(|(_, dom, _)| *dom)
    }
    
    /// Get transitivity for a verb form
    pub fn get_transitivity(&self, word: &str) -> Option<Transitivity> {
        self.lookup.get(&word.to_lowercase()).map(|(_, _, trans)| *trans)
    }
    
    /// Get total number of recognized verb forms
    pub fn form_count(&self) -> usize {
        self.lookup.len()
    }
}

/// Alias for backward compatibility
pub type VerbMorphology = VerbLexicon;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_count() {
        let lexicon = VerbLexicon::new();
        assert!(lexicon.form_count() > 500, 
            "Expected 500+ verb forms, got {}", lexicon.form_count());
    }

    #[test]
    fn test_is_verb() {
        let lexicon = VerbLexicon::new();
        
        assert!(lexicon.is_verb("kill"));
        assert!(lexicon.is_verb("killed"));
        assert!(lexicon.is_verb("killing"));
        assert!(lexicon.is_verb("kills"));
        
        assert!(!lexicon.is_verb("wizard"));
        assert!(!lexicon.is_verb("castle"));
    }

    #[test]
    fn test_get_relation_basic() {
        let lexicon = VerbLexicon::new();
        
        assert_eq!(lexicon.get_relation("kill"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("kills"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("killed"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("killing"), Some("KILLED"));
    }

    #[test]
    fn test_get_relation_synonyms() {
        let lexicon = VerbLexicon::new();
        
        assert_eq!(lexicon.get_relation("slay"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("slew"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("murder"), Some("KILLED"));
    }

    #[test]
    fn test_irregular_verb_forms() {
        let lexicon = VerbLexicon::new();
        
        // slay/slew/slain
        assert!(lexicon.is_verb("slay"));
        assert!(lexicon.is_verb("slew"));
        assert!(lexicon.is_verb("slain"));
        
        // fight/fought
        assert!(lexicon.is_verb("fight"));
        assert!(lexicon.is_verb("fought"));
        assert_eq!(lexicon.get_relation("fought"), Some("FOUGHT"));
    }

    #[test]
    fn test_case_insensitive() {
        let lexicon = VerbLexicon::new();
        
        assert_eq!(lexicon.get_relation("KILL"), Some("KILLED"));
        assert_eq!(lexicon.get_relation("Kill"), Some("KILLED"));
    }
}
