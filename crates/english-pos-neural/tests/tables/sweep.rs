//! Shared sweep gold table (see parent test header for provenance).
//! Included via #[path]; never hand-edited.

#[rustfmt::skip]
pub const SENTENCES: &[(&str, &str, &[&str], &[&str])] = &[
    // austen p1s0
    ("austen", "p1s0",
     &["It", "is", "a", "truth", "universally", "acknowledged", "that", "a", "single", "man", "in", "possession", "of", "a", "good", "fortune", "must", "be", "in", "want", "of", "a", "wife"],
     &["PRON", "AUX", "DET", "NOUN", "ADV", "VERB", "SCONJ", "DET", "ADJ", "NOUN", "ADP", "NOUN", "ADP", "DET", "ADJ", "NOUN", "AUX", "AUX", "ADP", "NOUN", "ADP", "DET", "NOUN"]),
    // austen p303s2
    ("austen", "p303s2",
     &["The", "note", "was", "immediately", "despatched", "and", "its", "contents", "as", "quickly", "complied", "with"],
     &["DET", "NOUN", "AUX", "ADV", "VERB", "CCONJ", "PRON", "NOUN", "ADV", "ADV", "VERB", "ADP"]),
    // austen p548s2
    ("austen", "p548s2",
     &["He", "had", "not", "a", "temper", "to", "bear", "the", "sort", "of", "competition", "in", "which", "we", "stood", "--", "the", "sort", "of", "preference", "which", "was", "often", "given", "me"],
     &["PRON", "AUX", "PART", "DET", "NOUN", "PART", "VERB", "DET", "NOUN", "ADP", "NOUN", "ADP", "PRON", "PRON", "VERB", "PUNCT", "DET", "NOUN", "ADP", "NOUN", "PRON", "AUX", "ADV", "VERB", "PRON"]),
    // austen p760s1
    ("austen", "p760s1",
     &["Jane", "'s", "temper", "was", "not", "desponding", ";", "and", "she", "was", "gradually", "led", "to", "hope", "though", "the", "diffidence", "of", "affection", "sometimes", "overcame", "the", "hope", "that", "Bingley", "would", "return", "to", "Netherfield", "and", "answer", "every", "wish", "of", "her", "heart"],
     &["PROPN", "PART", "NOUN", "AUX", "PART", "VERB", "PUNCT", "CCONJ", "PRON", "AUX", "ADV", "VERB", "ADP", "NOUN", "SCONJ", "DET", "NOUN", "ADP", "NOUN", "ADV", "VERB", "DET", "NOUN", "SCONJ", "PROPN", "AUX", "VERB", "ADP", "PROPN", "CCONJ", "VERB", "DET", "NOUN", "ADP", "PRON", "NOUN"]),
    // austen p950s0
    ("austen", "p950s0",
     &["“", "Do", "not", "make", "yourself", "uneasy", "my", "dear", "cousin", "about", "your", "apparel"],
     &["PUNCT", "AUX", "PART", "VERB", "PRON", "ADJ", "PRON", "ADJ", "NOUN", "ADP", "PRON", "NOUN"]),
    // austen p1148s35
    ("austen", "p1148s35",
     &["To", "persuade", "him", "against", "returning", "into", "Hertfordshire", "when", "that", "conviction", "had", "been", "given", "was", "scarcely", "the", "work", "of", "a", "moment"],
     &["PART", "VERB", "PRON", "ADP", "VERB", "ADP", "PROPN", "ADV", "DET", "NOUN", "AUX", "AUX", "VERB", "AUX", "ADV", "DET", "NOUN", "ADP", "DET", "NOUN"]),
    // austen p1328s1
    ("austen", "p1328s1",
     &["It", "was", "impossible", "for", "her", "to", "see", "the", "word", "without", "thinking", "of", "Pemberley", "and", "its", "owner"],
     &["PRON", "AUX", "ADJ", "ADP", "PRON", "PART", "VERB", "DET", "NOUN", "ADP", "VERB", "ADP", "PROPN", "CCONJ", "PRON", "NOUN"]),
    // austen p1484s0
    ("austen", "p1484s0",
     &["As", "he", "quitted", "the", "room", "Elizabeth", "felt", "how", "improbable", "it", "was", "that", "they", "should", "ever", "see", "each", "other", "again", "on", "such", "terms", "of", "cordiality", "as", "had", "marked", "their", "several", "meetings", "in", "Derbyshire", ";", "and", "as", "she", "threw", "a", "retrospective", "glance", "over", "the", "whole", "of", "their", "acquaintance", "so", "full", "of", "contradictions", "and", "varieties", "sighed", "at", "the", "perverseness", "of", "those", "feelings", "which", "would", "now", "have", "promoted", "its", "continuance", "and", "would", "formerly", "have", "rejoiced", "in", "its", "termination"],
     &["SCONJ", "PRON", "VERB", "DET", "NOUN", "PROPN", "VERB", "ADV", "ADJ", "PRON", "AUX", "SCONJ", "PRON", "AUX", "ADV", "VERB", "DET", "ADJ", "ADV", "ADP", "DET", "NOUN", "ADP", "NOUN", "SCONJ", "AUX", "VERB", "PRON", "ADJ", "NOUN", "ADP", "PROPN", "PUNCT", "CCONJ", "SCONJ", "PRON", "VERB", "DET", "ADJ", "NOUN", "ADP", "DET", "ADJ", "ADP", "PRON", "NOUN", "ADV", "ADJ", "ADP", "NOUN", "CCONJ", "NOUN", "VERB", "ADP", "DET", "NOUN", "ADP", "DET", "NOUN", "PRON", "AUX", "ADV", "AUX", "VERB", "PRON", "NOUN", "CCONJ", "AUX", "ADV", "AUX", "VERB", "ADP", "PRON", "NOUN"]),
    // austen p1689s1
    ("austen", "p1689s1",
     &["Lydia", "was", "Lydia", "still", ";", "untamed", "unabashed", "wild", "noisy", "and", "fearless"],
     &["PROPN", "AUX", "PROPN", "ADV", "PUNCT", "ADJ", "ADJ", "ADJ", "ADJ", "CCONJ", "ADJ"]),
    // austen p1908s0
    ("austen", "p1908s0",
     &["Mary", "petitioned", "for", "the", "use", "of", "the", "library", "at", "Netherfield", ";", "and", "Kitty", "begged", "very", "hard", "for", "a", "few", "balls", "there", "every", "winter"],
     &["PROPN", "VERB", "ADP", "DET", "NOUN", "ADP", "DET", "NOUN", "ADP", "PROPN", "PUNCT", "CCONJ", "PROPN", "VERB", "ADV", "ADV", "ADP", "DET", "ADJ", "NOUN", "ADV", "DET", "NOUN"]),
    // austen p105s0
    ("austen", "p105s0",
     &["His", "sisters", "were", "very", "anxious", "for", "his", "having", "an", "estate", "of", "his", "own", ";", "but", "though", "he", "was", "now", "established", "only", "as", "a", "tenant", "Miss", "Bingley", "was", "by", "no", "means", "unwilling", "to", "preside", "at", "his", "table", ";", "nor", "was", "Mrs.", "Hurst", "who", "had", "married", "a", "man", "of", "more", "fashion", "than", "fortune", "less", "disposed", "to", "consider", "his", "house", "as", "her", "home", "when", "it", "suited", "her"],
     &["PRON", "NOUN", "AUX", "ADV", "ADJ", "ADP", "PRON", "VERB", "DET", "NOUN", "ADP", "PRON", "ADJ", "PUNCT", "CCONJ", "SCONJ", "PRON", "AUX", "ADV", "VERB", "ADV", "ADP", "DET", "NOUN", "PROPN", "PROPN", "AUX", "ADP", "DET", "NOUN", "ADJ", "PART", "VERB", "ADP", "PRON", "NOUN", "PUNCT", "CCONJ", "AUX", "PROPN", "PROPN", "PRON", "AUX", "VERB", "DET", "NOUN", "ADP", "ADJ", "NOUN", "ADP", "NOUN", "ADV", "ADJ", "PART", "VERB", "PRON", "NOUN", "ADP", "PRON", "NOUN", "ADV", "PRON", "VERB", "PRON"]),
    // austen p372s1
    ("austen", "p372s1",
     &["But", "I", "am", "afraid", "you", "are", "giving", "it", "a", "turn", "which", "that", "gentleman", "did", "by", "no", "means", "intend", ";", "for", "he", "would", "certainly", "think", "the", "better", "of", "me", "if", "under", "such", "a", "circumstance", "I", "were", "to", "give", "a", "flat", "denial", "and", "ride", "off", "as", "fast", "as", "I", "could"],
     &["CCONJ", "PRON", "AUX", "ADJ", "PRON", "AUX", "VERB", "PRON", "DET", "NOUN", "PRON", "DET", "NOUN", "AUX", "ADP", "DET", "NOUN", "VERB", "PUNCT", "SCONJ", "PRON", "AUX", "ADV", "VERB", "DET", "ADJ", "ADP", "PRON", "SCONJ", "ADP", "DET", "DET", "NOUN", "PRON", "AUX", "PART", "VERB", "DET", "ADJ", "NOUN", "CCONJ", "VERB", "ADV", "ADV", "ADV", "SCONJ", "PRON", "AUX"]),
    // austen p1016s0
    ("austen", "p1016s0",
     &["When", "coffee", "was", "over", "Colonel", "Fitzwilliam", "reminded", "Elizabeth", "of", "having", "promised", "to", "play", "to", "him", ";", "and", "she", "sat", "down", "directly", "to", "the", "instrument"],
     &["ADV", "NOUN", "AUX", "ADV", "PROPN", "PROPN", "VERB", "PROPN", "ADP", "VERB", "VERB", "PART", "VERB", "ADP", "PRON", "PUNCT", "CCONJ", "PRON", "VERB", "ADV", "ADV", "ADP", "DET", "NOUN"]),
    // austen p1326s2
    ("austen", "p1326s2",
     &["In", "that", "county", "there", "was", "enough", "to", "be", "seen", "to", "occupy", "the", "chief", "of", "their", "three", "weeks", ";", "and", "to", "Mrs.", "Gardiner", "it", "had", "a", "peculiarly", "strong", "attraction"],
     &["ADP", "DET", "NOUN", "PRON", "VERB", "ADJ", "PART", "AUX", "VERB", "PART", "VERB", "DET", "NOUN", "ADP", "PRON", "NUM", "NOUN", "PUNCT", "CCONJ", "ADP", "PROPN", "PROPN", "PRON", "VERB", "DET", "ADV", "ADJ", "NOUN"]),
    // austen p106s1
    ("austen", "p106s1",
     &["Bingley", "was", "endeared", "to", "Darcy", "by", "the", "easiness", "openness", "and", "ductility", "of", "his", "temper", "though", "no", "disposition", "could", "offer", "a", "greater", "contrast", "to", "his", "own", "and", "though", "with", "his", "own", "he", "never", "appeared", "dissatisfied"],
     &["PROPN", "AUX", "VERB", "ADP", "PROPN", "ADP", "DET", "NOUN", "NOUN", "CCONJ", "NOUN", "ADP", "PRON", "NOUN", "SCONJ", "DET", "NOUN", "AUX", "VERB", "DET", "ADJ", "NOUN", "ADP", "PRON", "ADJ", "CCONJ", "SCONJ", "ADP", "PRON", "ADJ", "PRON", "ADV", "VERB", "ADJ"]),
    // austen p450s1
    ("austen", "p450s1",
     &["Miss", "Bingley", "'s", "civility", "to", "Elizabeth", "increased", "at", "last", "very", "rapidly", "as", "well", "as", "her", "affection", "for", "Jane", ";", "and", "when", "they", "parted", "after", "assuring", "the", "latter", "of", "the", "pleasure", "it", "would", "always", "give", "her", "to", "see", "her", "either", "at", "Longbourn", "or", "Netherfield", "and", "embracing", "her", "most", "tenderly", "she", "even", "shook", "hands", "with", "the", "former"],
     &["PROPN", "PROPN", "PART", "NOUN", "ADP", "PROPN", "VERB", "ADP", "ADJ", "ADV", "ADV", "ADV", "ADV", "ADP", "PRON", "NOUN", "ADP", "PROPN", "PUNCT", "CCONJ", "ADV", "PRON", "VERB", "ADP", "VERB", "DET", "ADJ", "ADP", "DET", "NOUN", "PRON", "AUX", "ADV", "VERB", "PRON", "PART", "VERB", "PRON", "CCONJ", "ADP", "PROPN", "CCONJ", "PROPN", "CCONJ", "VERB", "PRON", "ADV", "ADV", "PRON", "ADV", "VERB", "NOUN", "ADP", "DET", "ADJ"]),
    // austen p663s0
    ("austen", "p663s0",
     &["At", "length", "however", "Mrs.", "Bennet", "had", "no", "more", "to", "say", ";", "and", "Lady", "Lucas", "who", "had", "been", "long", "yawning", "at", "the", "repetition", "of", "delights", "which", "she", "saw", "no", "likelihood", "of", "sharing", "was", "left", "to", "the", "comforts", "of", "cold", "ham", "and", "chicken"],
     &["ADP", "NOUN", "ADV", "PROPN", "PROPN", "VERB", "DET", "ADJ", "PART", "VERB", "PUNCT", "CCONJ", "PROPN", "PROPN", "PRON", "AUX", "AUX", "ADV", "VERB", "ADP", "DET", "NOUN", "ADP", "NOUN", "PRON", "PRON", "VERB", "DET", "NOUN", "ADP", "VERB", "AUX", "VERB", "ADP", "DET", "NOUN", "ADP", "ADJ", "NOUN", "CCONJ", "NOUN"]),
    // austen p692s2
    ("austen", "p692s2",
     &["My", "situation", "in", "life", "my", "connections", "with", "the", "family", "of", "De", "Bourgh", "and", "my", "relationship", "to", "your", "own", "are", "circumstances", "highly", "in", "my", "favour", ";", "and", "you", "should", "take", "it", "into", "further", "consideration", "that", "in", "spite", "of", "your", "manifold", "attractions", "it", "is", "by", "no", "means", "certain", "that", "another", "offer", "of", "marriage", "may", "ever", "be", "made", "you"],
     &["PRON", "NOUN", "ADP", "NOUN", "PRON", "NOUN", "ADP", "DET", "NOUN", "ADP", "PROPN", "PROPN", "CCONJ", "PRON", "NOUN", "ADP", "PRON", "ADJ", "AUX", "NOUN", "ADV", "ADP", "PRON", "NOUN", "PUNCT", "CCONJ", "PRON", "AUX", "VERB", "PRON", "ADP", "ADJ", "NOUN", "SCONJ", "ADP", "NOUN", "ADP", "PRON", "ADJ", "NOUN", "PRON", "AUX", "ADP", "DET", "NOUN", "ADJ", "SCONJ", "DET", "NOUN", "ADP", "NOUN", "AUX", "ADV", "AUX", "VERB", "PRON"]),
    // austen p731s3
    ("austen", "p731s3",
     &["Perhaps", "not", "the", "less", "so", "from", "feeling", "a", "doubt", "of", "my", "positive", "happiness", "had", "my", "fair", "cousin", "honoured", "me", "with", "her", "hand", ";", "for", "I", "have", "often", "observed", "that", "resignation", "is", "never", "so", "perfect", "as", "when", "the", "blessing", "denied", "begins", "to", "lose", "somewhat", "of", "its", "value", "in", "our", "estimation"],
     &["ADV", "PART", "DET", "ADV", "ADV", "ADP", "VERB", "DET", "NOUN", "ADP", "PRON", "ADJ", "NOUN", "AUX", "PRON", "ADJ", "NOUN", "VERB", "PRON", "ADP", "PRON", "NOUN", "PUNCT", "SCONJ", "PRON", "AUX", "ADV", "VERB", "SCONJ", "NOUN", "AUX", "ADV", "ADV", "ADJ", "SCONJ", "ADV", "DET", "NOUN", "VERB", "VERB", "PART", "VERB", "ADV", "ADP", "PRON", "NOUN", "ADP", "PRON", "NOUN"]),
    // austen p1147s0
    ("austen", "p1147s0",
     &["“", "Be", "not", "alarmed", "madam", "on", "receiving", "this", "letter", "by", "the", "apprehension", "of", "its", "containing", "any", "repetition", "of", "those", "sentiments", "or", "renewal", "of", "those", "offers", "which", "were", "last", "night", "so", "disgusting", "to", "you"],
     &["PUNCT", "VERB", "PART", "VERB", "NOUN", "ADP", "VERB", "DET", "NOUN", "ADP", "DET", "NOUN", "ADP", "PRON", "VERB", "DET", "NOUN", "ADP", "DET", "NOUN", "CCONJ", "NOUN", "ADP", "DET", "NOUN", "PRON", "AUX", "ADJ", "NOUN", "ADV", "ADJ", "ADP", "PRON"]),
    // doyle p0s2
    ("doyle", "p0s2",
     &["In", "his", "eyes", "she", "eclipses", "and", "predominates", "the", "whole", "of", "her", "sex"],
     &["ADP", "PRON", "NOUN", "PRON", "VERB", "CCONJ", "VERB", "DET", "ADJ", "ADP", "PRON", "NOUN"]),
    // doyle p310s1
    ("doyle", "p310s1",
     &["You", "see", "it", "is", "really", "confined", "to", "Londoners", "and", "to", "grown", "men"],
     &["PRON", "VERB", "PRON", "AUX", "ADV", "VERB", "ADP", "PROPN", "CCONJ", "ADP", "ADJ", "NOUN"]),
    // doyle p550s0
    ("doyle", "p550s0",
     &["For", "all", "the", "preposterous", "hat", "and", "the", "vacuous", "face", "there", "was", "something", "noble", "in", "the", "simple", "faith", "of", "our", "visitor", "which", "compelled", "our", "respect"],
     &["ADP", "DET", "DET", "ADJ", "NOUN", "CCONJ", "DET", "ADJ", "NOUN", "PRON", "VERB", "PRON", "ADJ", "ADP", "DET", "ADJ", "NOUN", "ADP", "PRON", "NOUN", "PRON", "VERB", "PRON", "NOUN"]),
    // doyle p742s23
    ("doyle", "p742s23",
     &["He", "ran", "up", "and", "down", "sometimes", "losing", "sometimes", "finding", "the", "track", "until", "we", "were", "well", "within", "the", "edge", "of", "the", "wood", "and", "under", "the", "shadow", "of", "a", "great", "beech", "the", "largest", "tree", "in", "the", "neighbourhood"],
     &["PRON", "VERB", "ADV", "CCONJ", "ADV", "ADV", "VERB", "ADV", "VERB", "DET", "NOUN", "SCONJ", "PRON", "AUX", "ADV", "ADP", "DET", "NOUN", "ADP", "DET", "NOUN", "CCONJ", "ADP", "DET", "NOUN", "ADP", "DET", "ADJ", "NOUN", "DET", "ADJ", "NOUN", "ADP", "DET", "NOUN"]),
    // doyle p1011s1
    ("doyle", "p1011s1",
     &["She", "had", "the", "surest", "information", "that", "of", "late", "he", "had", "when", "the", "fit", "was", "on", "him", "made", "use", "of", "an", "opium", "den", "in", "the", "farthest", "east", "of", "the", "City"],
     &["PRON", "VERB", "DET", "ADJ", "NOUN", "SCONJ", "ADP", "ADV", "PRON", "VERB", "ADV", "DET", "NOUN", "AUX", "ADP", "PRON", "VERB", "NOUN", "ADP", "DET", "NOUN", "NOUN", "ADP", "DET", "ADJ", "NOUN", "ADP", "DET", "PROPN"]),
    // doyle p1281s3
    ("doyle", "p1281s3",
     &["James", "Ryder", "upper-attendant", "at", "the", "hotel", "gave", "his", "evidence", "to", "the", "effect", "that", "he", "had", "shown", "Horner", "up", "to", "the", "dressing-room", "of", "the", "Countess", "of", "Morcar", "upon", "the", "day", "of", "the", "robbery", "in", "order", "that", "he", "might", "solder", "the", "second", "bar", "of", "the", "grate", "which", "was", "loose"],
     &["PROPN", "PROPN", "NOUN", "ADP", "DET", "NOUN", "VERB", "PRON", "NOUN", "ADP", "DET", "NOUN", "SCONJ", "PRON", "AUX", "VERB", "PROPN", "ADP", "ADP", "DET", "NOUN", "ADP", "DET", "PROPN", "ADP", "PROPN", "ADP", "DET", "NOUN", "ADP", "DET", "NOUN", "ADP", "NOUN", "SCONJ", "PRON", "AUX", "VERB", "DET", "ADJ", "NOUN", "ADP", "DET", "NOUN", "PRON", "AUX", "ADJ"]),
    // doyle p1547s7
    ("doyle", "p1547s7",
     &["I", "should", "be", "very", "much", "obliged", "if", "you", "would", "slip", "your", "revolver", "into", "your", "pocket"],
     &["PRON", "AUX", "AUX", "ADV", "ADV", "VERB", "SCONJ", "PRON", "AUX", "VERB", "PRON", "NOUN", "ADP", "PRON", "NOUN"]),
    // doyle p1802s1
    ("doyle", "p1802s1",
     &["His", "face", "set", "hard", "and", "a", "baleful", "light", "sprang", "up", "in", "his", "grey", "eyes"],
     &["PRON", "NOUN", "VERB", "ADJ", "CCONJ", "DET", "ADJ", "NOUN", "VERB", "ADV", "ADP", "PRON", "ADJ", "NOUN"]),
    // doyle p2077s7
    ("doyle", "p2077s7",
     &["Then", "who", "could", "this", "American", "be", "and", "why", "should", "he", "possess", "so", "much", "influence", "over", "her"],
     &["ADV", "PRON", "AUX", "DET", "NOUN", "AUX", "CCONJ", "ADV", "AUX", "PRON", "VERB", "ADV", "ADJ", "NOUN", "ADP", "PRON"]),
    // doyle p2302s0
    ("doyle", "p2302s0",
     &["“", "A", "day", "which", "has", "saved", "England", "from", "a", "great", "public", "scandal", "”", "said", "the", "banker", "rising"],
     &["PUNCT", "DET", "NOUN", "PRON", "AUX", "VERB", "PROPN", "ADP", "DET", "ADJ", "ADJ", "NOUN", "PUNCT", "VERB", "DET", "NOUN", "NOUN"]),
    // doyle p527s1
    ("doyle", "p527s1",
     &["It", "was", "to", "be", "at", "St.", "Saviour", "'s", "near", "King", "'s", "Cross", "and", "we", "were", "to", "have", "breakfast", "afterwards", "at", "the", "St.", "Pancras", "Hotel"],
     &["PRON", "AUX", "PART", "AUX", "ADP", "PROPN", "PROPN", "PART", "ADP", "PROPN", "PART", "PROPN", "CCONJ", "PRON", "AUX", "PART", "VERB", "NOUN", "ADV", "ADP", "DET", "PROPN", "PROPN", "PROPN"]),
    // doyle p639s4
    ("doyle", "p639s4",
     &["As", "to", "his", "remark", "about", "his", "deserts", "it", "was", "also", "not", "unnatural", "if", "you", "consider", "that", "he", "stood", "beside", "the", "dead", "body", "of", "his", "father", "and", "that", "there", "is", "no", "doubt", "that", "he", "had", "that", "very", "day", "so", "far", "forgotten", "his", "filial", "duty", "as", "to", "bandy", "words", "with", "him", "and", "even", "according", "to", "the", "little", "girl", "whose", "evidence", "is", "so", "important", "to", "raise", "his", "hand", "as", "if", "to", "strike", "him"],
     &["ADP", "ADP", "PRON", "NOUN", "ADP", "PRON", "NOUN", "PRON", "AUX", "ADV", "PART", "ADJ", "SCONJ", "PRON", "VERB", "SCONJ", "PRON", "VERB", "ADP", "DET", "ADJ", "NOUN", "ADP", "PRON", "NOUN", "CCONJ", "SCONJ", "PRON", "VERB", "DET", "NOUN", "SCONJ", "PRON", "AUX", "DET", "ADV", "NOUN", "ADV", "ADV", "VERB", "PRON", "ADJ", "NOUN", "SCONJ", "PART", "VERB", "NOUN", "ADP", "PRON", "CCONJ", "ADV", "ADP", "ADP", "DET", "ADJ", "NOUN", "PRON", "NOUN", "AUX", "ADV", "ADJ", "PART", "VERB", "PRON", "NOUN", "SCONJ", "SCONJ", "PART", "VERB", "PRON"]),
    // doyle p1650s5
    ("doyle", "p1650s5",
     &["But", "we", "shall", "have", "horrors", "enough", "before", "the", "night", "is", "over", ";", "for", "goodness", "’", "sake", "let", "us", "have", "a", "quiet", "pipe", "and", "turn", "our", "minds", "for", "a", "few", "hours", "to", "something", "more", "cheerful"],
     &["CCONJ", "PRON", "AUX", "VERB", "NOUN", "ADV", "ADP", "DET", "NOUN", "AUX", "ADV", "PUNCT", "ADP", "NOUN", "PART", "NOUN", "VERB", "PRON", "VERB", "DET", "ADJ", "NOUN", "CCONJ", "VERB", "PRON", "NOUN", "ADP", "DET", "ADJ", "NOUN", "ADP", "PRON", "ADJ", "ADJ"]),
    // doyle p362s2
    ("doyle", "p362s2",
     &["From", "what", "you", "have", "told", "me", "I", "think", "that", "it", "is", "possible", "that", "graver", "issues", "hang", "from", "it", "than", "might", "at", "first", "sight", "appear"],
     &["ADP", "PRON", "PRON", "AUX", "VERB", "PRON", "PRON", "VERB", "SCONJ", "PRON", "AUX", "ADJ", "SCONJ", "ADJ", "NOUN", "VERB", "ADP", "PRON", "SCONJ", "AUX", "ADP", "ADJ", "NOUN", "VERB"]),
    // doyle p529s2
    ("doyle", "p529s2",
     &["Why", "all", "the", "morning", "he", "was", "saying", "to", "me", "that", "whatever", "happened", "I", "was", "to", "be", "true", ";", "and", "that", "even", "if", "something", "quite", "unforeseen", "occurred", "to", "separate", "us", "I", "was", "always", "to", "remember", "that", "I", "was", "pledged", "to", "him", "and", "that", "he", "would", "claim", "his", "pledge", "sooner", "or", "later"],
     &["ADV", "DET", "DET", "NOUN", "PRON", "AUX", "VERB", "ADP", "PRON", "SCONJ", "PRON", "VERB", "PRON", "AUX", "PART", "AUX", "ADJ", "PUNCT", "CCONJ", "SCONJ", "ADV", "SCONJ", "PRON", "ADV", "ADJ", "VERB", "PART", "VERB", "PRON", "PRON", "AUX", "ADV", "PART", "VERB", "SCONJ", "PRON", "AUX", "ADJ", "ADP", "PRON", "CCONJ", "SCONJ", "PRON", "AUX", "VERB", "PRON", "NOUN", "ADV", "CCONJ", "ADV"]),
    // doyle p1235s2
    ("doyle", "p1235s2",
     &["Its", "finder", "has", "carried", "it", "off", "therefore", "to", "fulfil", "the", "ultimate", "destiny", "of", "a", "goose", "while", "I", "continue", "to", "retain", "the", "hat", "of", "the", "unknown", "gentleman", "who", "lost", "his", "Christmas", "dinner"],
     &["PRON", "NOUN", "AUX", "VERB", "PRON", "ADV", "ADV", "PART", "VERB", "DET", "ADJ", "NOUN", "ADP", "DET", "NOUN", "SCONJ", "PRON", "VERB", "PART", "VERB", "DET", "NOUN", "ADP", "DET", "ADJ", "NOUN", "PRON", "VERB", "PRON", "PROPN", "NOUN"]),
    // doyle p1281s8
    ("doyle", "p1281s8",
     &["Inspector", "Bradstreet", "B", "division", "gave", "evidence", "as", "to", "the", "arrest", "of", "Horner", "who", "struggled", "frantically", "and", "protested", "his", "innocence", "in", "the", "strongest", "terms"],
     &["PROPN", "PROPN", "PROPN", "NOUN", "VERB", "NOUN", "ADP", "ADP", "DET", "NOUN", "ADP", "PROPN", "PRON", "VERB", "ADV", "CCONJ", "VERB", "PRON", "NOUN", "ADP", "DET", "ADJ", "NOUN"]),
    // doyle p1443s2
    ("doyle", "p1443s2",
     &["She", "raised", "her", "veil", "as", "she", "spoke", "and", "we", "could", "see", "that", "she", "was", "indeed", "in", "a", "pitiable", "state", "of", "agitation", "her", "face", "all", "drawn", "and", "grey", "with", "restless", "frightened", "eyes", "like", "those", "of", "some", "hunted", "animal"],
     &["PRON", "VERB", "PRON", "NOUN", "SCONJ", "PRON", "VERB", "CCONJ", "PRON", "AUX", "VERB", "SCONJ", "PRON", "AUX", "ADV", "ADP", "DET", "ADJ", "NOUN", "ADP", "NOUN", "PRON", "NOUN", "ADV", "VERB", "CCONJ", "ADJ", "ADP", "ADJ", "ADJ", "NOUN", "ADP", "DET", "ADP", "DET", "ADJ", "NOUN"]),
    // doyle p1679s1
    ("doyle", "p1679s1",
     &["It", "is", "not", "necessary", "that", "I", "should", "prolong", "a", "narrative", "which", "has", "already", "run", "to", "too", "great", "a", "length", "by", "telling", "how", "we", "broke", "the", "sad", "news", "to", "the", "terrified", "girl", "how", "we", "conveyed", "her", "by", "the", "morning", "train", "to", "the", "care", "of", "her", "good", "aunt", "at", "Harrow", "of", "how", "the", "slow", "process", "of", "official", "inquiry", "came", "to", "the", "conclusion", "that", "the", "doctor", "met", "his", "fate", "while", "indiscreetly", "playing", "with", "a", "dangerous", "pet"],
     &["PRON", "AUX", "PART", "ADJ", "SCONJ", "PRON", "AUX", "VERB", "DET", "NOUN", "PRON", "AUX", "ADV", "VERB", "ADP", "ADV", "ADJ", "NOUN", "NOUN", "ADP", "VERB", "ADV", "PRON", "VERB", "DET", "ADJ", "NOUN", "ADP", "DET", "ADJ", "NOUN", "ADV", "PRON", "VERB", "PRON", "ADP", "DET", "NOUN", "NOUN", "ADP", "DET", "NOUN", "ADP", "PRON", "ADJ", "NOUN", "ADP", "PROPN", "ADP", "ADV", "DET", "ADJ", "NOUN", "ADP", "ADJ", "NOUN", "VERB", "ADP", "DET", "NOUN", "SCONJ", "DET", "NOUN", "VERB", "PRON", "NOUN", "SCONJ", "ADV", "VERB", "ADP", "DET", "ADJ", "NOUN"]),
    // doyle p1766s1
    ("doyle", "p1766s1",
     &["The", "only", "point", "which", "I", "could", "not", "quite", "understand", "was", "what", "use", "you", "could", "make", "of", "a", "hydraulic", "press", "in", "excavating", "fuller", "'s-earth", "which", "as", "I", "understand", "is", "dug", "out", "like", "gravel", "from", "a", "pit"],
     &["DET", "ADJ", "NOUN", "PRON", "PRON", "AUX", "PART", "ADV", "VERB", "AUX", "PRON", "NOUN", "PRON", "AUX", "VERB", "ADP", "DET", "ADJ", "NOUN", "ADP", "VERB", "NOUN", "NOUN", "PRON", "SCONJ", "PRON", "VERB", "AUX", "VERB", "ADV", "ADP", "NOUN", "ADP", "DET", "NOUN"]),
    // stevenson p1s0
    ("stevenson", "p1s0",
     &["I", "remember", "him", "as", "if", "it", "were", "yesterday", "as", "he", "came", "plodding", "to", "the", "inn", "door", "his", "sea-chest", "following", "behind", "him", "in", "a", "hand-barrow", "--", "a", "tall", "strong", "heavy", "nut-brown", "man", "his", "tarry", "pigtail", "falling", "over", "the", "shoulder", "of", "his", "soiled", "blue", "coat", "his", "hands", "ragged", "and", "scarred", "with", "black", "broken", "nails", "and", "the", "sabre", "cut", "across", "one", "cheek", "a", "dirty", "livid", "white"],
     &["PRON", "VERB", "PRON", "SCONJ", "SCONJ", "PRON", "AUX", "NOUN", "SCONJ", "PRON", "VERB", "VERB", "ADP", "DET", "NOUN", "NOUN", "PRON", "NOUN", "VERB", "ADP", "PRON", "ADP", "DET", "NOUN", "PUNCT", "DET", "ADJ", "ADJ", "ADJ", "ADJ", "NOUN", "PRON", "ADJ", "NOUN", "VERB", "ADP", "DET", "NOUN", "ADP", "PRON", "ADJ", "ADJ", "NOUN", "PRON", "NOUN", "ADJ", "CCONJ", "ADJ", "ADP", "ADJ", "ADJ", "NOUN", "CCONJ", "DET", "NOUN", "NOUN", "ADP", "NUM", "NOUN", "DET", "ADJ", "ADJ", "NOUN"]),
    // stevenson p111s1
    ("stevenson", "p111s1",
     &["She", "would", "not", "she", "declared", "lose", "money", "that", "belonged", "to", "her", "fatherless", "boy", ";", "“", "If", "none", "of", "the", "rest", "of", "you", "dare", "”", "she", "said", "“", "Jim", "and", "I", "dare"],
     &["PRON", "AUX", "PART", "PRON", "VERB", "VERB", "NOUN", "PRON", "VERB", "ADP", "PRON", "ADJ", "NOUN", "PUNCT", "PUNCT", "SCONJ", "PRON", "ADP", "DET", "NOUN", "ADP", "PRON", "VERB", "PUNCT", "PRON", "VERB", "PUNCT", "PROPN", "CCONJ", "PRON", "VERB"]),
    // stevenson p259s2
    ("stevenson", "p259s2",
     &["There", "was", "a", "street", "on", "each", "side", "and", "an", "open", "door", "on", "both", "which", "made", "the", "large", "low", "room", "pretty", "clear", "to", "see", "in", "in", "spite", "of", "clouds", "of", "tobacco", "smoke"],
     &["PRON", "VERB", "DET", "NOUN", "ADP", "DET", "NOUN", "CCONJ", "DET", "ADJ", "NOUN", "ADP", "DET", "PRON", "VERB", "DET", "ADJ", "ADJ", "NOUN", "ADV", "ADJ", "PART", "VERB", "ADP", "ADP", "NOUN", "ADP", "NOUN", "ADP", "NOUN", "NOUN"]),
    // stevenson p418s0
    ("stevenson", "p418s0",
     &["“", "Why", "we", "'re", "all", "seamen", "aboard", "here", "I", "should", "think", "”", "said", "the", "lad", "Dick"],
     &["PUNCT", "ADV", "PRON", "AUX", "ADV", "NOUN", "ADV", "ADV", "PRON", "AUX", "VERB", "PUNCT", "VERB", "DET", "NOUN", "PROPN"]),
    // stevenson p548s2
    ("stevenson", "p548s2",
     &["Of", "all", "the", "beggar-men", "that", "I", "had", "seen", "or", "fancied", "he", "was", "the", "chief", "for", "raggedness"],
     &["ADP", "DET", "DET", "NOUN", "PRON", "PRON", "AUX", "VERB", "CCONJ", "VERB", "PRON", "AUX", "DET", "NOUN", "ADP", "NOUN"]),
    // stevenson p709s3
    ("stevenson", "p709s3",
     &["For", "four", "or", "five", "of", "them", "were", "busy", "carrying", "off", "our", "stores", "and", "wading", "out", "with", "them", "to", "one", "of", "the", "gigs", "that", "lay", "close", "by", "pulling", "an", "oar", "or", "so", "to", "hold", "her", "steady", "against", "the", "current"],
     &["ADP", "NUM", "CCONJ", "NUM", "ADP", "PRON", "AUX", "ADJ", "VERB", "ADV", "PRON", "NOUN", "CCONJ", "VERB", "ADV", "ADP", "PRON", "ADP", "NUM", "ADP", "DET", "NOUN", "PRON", "VERB", "ADJ", "ADV", "VERB", "DET", "NOUN", "CCONJ", "ADV", "PART", "VERB", "PRON", "ADJ", "ADP", "DET", "NOUN"]),
    // stevenson p868s0
    ("stevenson", "p868s0",
     &["All", "the", "time", "I", "was", "washing", "out", "the", "block", "house", "and", "then", "washing", "up", "the", "things", "from", "dinner", "this", "disgust", "and", "envy", "kept", "growing", "stronger", "and", "stronger", "till", "at", "last", "being", "near", "a", "bread-bag", "and", "no", "one", "then", "observing", "me", "I", "took", "the", "first", "step", "towards", "my", "escapade", "and", "filled", "both", "pockets", "of", "my", "coat", "with", "biscuit"],
     &["DET", "DET", "NOUN", "PRON", "AUX", "VERB", "ADV", "DET", "NOUN", "NOUN", "CCONJ", "ADV", "VERB", "ADV", "DET", "NOUN", "ADP", "NOUN", "DET", "NOUN", "CCONJ", "NOUN", "VERB", "VERB", "ADJ", "CCONJ", "ADJ", "SCONJ", "ADP", "ADJ", "VERB", "ADP", "DET", "NOUN", "CCONJ", "DET", "PRON", "ADV", "VERB", "PRON", "PRON", "VERB", "DET", "ADJ", "NOUN", "ADP", "PRON", "NOUN", "CCONJ", "VERB", "DET", "NOUN", "ADP", "PRON", "NOUN", "ADP", "NOUN"]),
    // stevenson p980s0
    ("stevenson", "p980s0",
     &["“", "Cap", "'n", "”", "said", "he", "at", "length", "with", "that", "same", "uncomfortable", "smile", "“", "here", "'s", "my", "old", "shipmate", "O", "'Brien", ";", "s", "'pose", "you", "was", "to", "heave", "him", "overboard"],
     &["PUNCT", "NOUN", "NOUN", "PUNCT", "VERB", "PRON", "ADP", "NOUN", "ADP", "DET", "ADJ", "ADJ", "NOUN", "PUNCT", "ADV", "AUX", "PRON", "ADJ", "NOUN", "INTJ", "PROPN", "PUNCT", "VERB", "VERB", "PRON", "AUX", "PART", "VERB", "PRON", "ADV"]),
    // stevenson p1077s8
    ("stevenson", "p1077s8",
     &["But", "one", "thing", "I", "'ll", "say", "and", "no", "more", ";", "if", "you", "spare", "me", "bygones", "are", "bygones", "and", "when", "you", "fellows", "are", "in", "court", "for", "piracy", "I", "'ll", "save", "you", "all", "I", "can"],
     &["CCONJ", "NUM", "NOUN", "PRON", "AUX", "VERB", "CCONJ", "DET", "ADV", "PUNCT", "SCONJ", "PRON", "VERB", "PRON", "NOUN", "AUX", "NOUN", "CCONJ", "ADV", "PRON", "NOUN", "AUX", "ADP", "NOUN", "ADP", "NOUN", "PRON", "AUX", "VERB", "PRON", "DET", "PRON", "AUX"]),
    // stevenson p1220s2
    ("stevenson", "p1220s2",
     &["The", "top", "of", "the", "plateau", "was", "dotted", "thickly", "with", "pine-trees", "of", "varying", "height"],
     &["DET", "NOUN", "ADP", "DET", "NOUN", "AUX", "VERB", "ADV", "ADP", "NOUN", "ADP", "ADJ", "NOUN"]),
    // stevenson p13s2
    ("stevenson", "p13s2",
     &["I", "followed", "him", "in", "and", "I", "remember", "observing", "the", "contrast", "the", "neat", "bright", "doctor", "with", "his", "powder", "as", "white", "as", "snow", "and", "his", "bright", "black", "eyes", "and", "pleasant", "manners", "made", "with", "the", "coltish", "country", "folk", "and", "above", "all", "with", "that", "filthy", "heavy", "bleared", "scarecrow", "of", "a", "pirate", "of", "ours", "sitting", "far", "gone", "in", "rum", "with", "his", "arms", "on", "the", "table"],
     &["PRON", "VERB", "PRON", "ADP", "CCONJ", "PRON", "VERB", "VERB", "DET", "NOUN", "DET", "ADJ", "ADJ", "NOUN", "ADP", "PRON", "NOUN", "ADV", "ADJ", "SCONJ", "NOUN", "CCONJ", "PRON", "ADJ", "ADJ", "NOUN", "CCONJ", "ADJ", "NOUN", "VERB", "ADP", "DET", "ADJ", "NOUN", "NOUN", "CCONJ", "ADP", "DET", "ADP", "DET", "ADJ", "ADJ", "ADJ", "NOUN", "ADP", "DET", "NOUN", "ADP", "PRON", "VERB", "ADV", "ADJ", "ADP", "NOUN", "ADP", "PRON", "NOUN", "ADP", "DET", "NOUN"]),
    // stevenson p164s2
    ("stevenson", "p164s2",
     &["And", "that", "was", "plainly", "the", "last", "signal", "of", "danger", "for", "the", "buccaneers", "turned", "at", "once", "and", "ran", "separating", "in", "every", "direction", "one", "seaward", "along", "the", "cove", "one", "slant", "across", "the", "hill", "and", "so", "on", "so", "that", "in", "half", "a", "minute", "not", "a", "sign", "of", "them", "remained", "but", "Pew"],
     &["CCONJ", "PRON", "AUX", "ADV", "DET", "ADJ", "NOUN", "ADP", "NOUN", "SCONJ", "DET", "NOUN", "VERB", "ADP", "ADV", "CCONJ", "VERB", "VERB", "ADP", "DET", "NOUN", "NUM", "ADV", "ADP", "DET", "NOUN", "NUM", "NOUN", "ADP", "DET", "NOUN", "CCONJ", "ADV", "ADV", "ADV", "SCONJ", "ADP", "DET", "DET", "NOUN", "PART", "DET", "NOUN", "ADP", "PRON", "VERB", "ADP", "PROPN"]),
    // stevenson p209s2
    ("stevenson", "p209s2",
     &["One", "was", "the", "same", "as", "the", "tattoo", "mark", "“", "Billy", "Bones", "his", "fancy", "”", ";", "then", "there", "was", "“", "Mr.", "W.", "Bones", "mate", "”", "“", "No", "more", "rum", "”", "“", "Off", "Palm", "Key", "he", "got", "itt", "”", "and", "some", "other", "snatches", "mostly", "single", "words", "and", "unintelligible"],
     &["PRON", "AUX", "DET", "ADJ", "ADP", "DET", "NOUN", "NOUN", "PUNCT", "PROPN", "PROPN", "PRON", "NOUN", "PUNCT", "PUNCT", "ADV", "PRON", "VERB", "PUNCT", "PROPN", "PROPN", "PROPN", "NOUN", "PUNCT", "PUNCT", "ADV", "ADV", "NOUN", "PUNCT", "PUNCT", "ADP", "PROPN", "PROPN", "PRON", "VERB", "PRON", "PUNCT", "CCONJ", "DET", "ADJ", "NOUN", "ADV", "ADJ", "NOUN", "CCONJ", "ADJ"]),
    // stevenson p226s2
    ("stevenson", "p226s2",
     &["These", "fellows", "who", "attacked", "the", "inn", "tonight", "--", "bold", "desperate", "blades", "for", "sure", "--", "and", "the", "rest", "who", "stayed", "aboard", "that", "lugger", "and", "more", "I", "dare", "say", "not", "far", "off", "are", "one", "and", "all", "through", "thick", "and", "thin", "bound", "that", "they", "'ll", "get", "that", "money"],
     &["DET", "NOUN", "PRON", "VERB", "DET", "NOUN", "NOUN", "PUNCT", "ADJ", "ADJ", "NOUN", "ADP", "ADJ", "PUNCT", "CCONJ", "DET", "NOUN", "PRON", "VERB", "ADV", "DET", "NOUN", "CCONJ", "ADJ", "PRON", "VERB", "VERB", "PART", "ADV", "ADP", "AUX", "NUM", "CCONJ", "DET", "ADP", "NOUN", "CCONJ", "ADJ", "ADJ", "SCONJ", "PRON", "AUX", "VERB", "DET", "NOUN"]),
    // stevenson p239s1
    ("stevenson", "p239s1",
     &["I", "found", "he", "was", "an", "old", "sailor", "kept", "a", "public-house", "knew", "all", "the", "seafaring", "men", "in", "Bristol", "had", "lost", "his", "health", "ashore", "and", "wanted", "a", "good", "berth", "as", "cook", "to", "get", "to", "sea", "again"],
     &["PRON", "VERB", "PRON", "AUX", "DET", "ADJ", "NOUN", "VERB", "DET", "NOUN", "VERB", "DET", "DET", "ADJ", "NOUN", "ADP", "PROPN", "AUX", "VERB", "PRON", "NOUN", "ADV", "CCONJ", "VERB", "DET", "ADJ", "NOUN", "ADP", "NOUN", "PART", "VERB", "ADP", "NOUN", "ADV"]),
    // stevenson p371s1
    ("stevenson", "p371s1",
     &["But", "soon", "the", "anchor", "was", "short", "up", ";", "soon", "it", "was", "hanging", "dripping", "at", "the", "bows", ";", "soon", "the", "sails", "began", "to", "draw", "and", "the", "land", "and", "shipping", "to", "flit", "by", "on", "either", "side", ";", "and", "before", "I", "could", "lie", "down", "to", "snatch", "an", "hour", "of", "slumber", "the", "HISPANIOLA", "had", "begun", "her", "voyage", "to", "the", "Isle", "of", "Treasure"],
     &["CCONJ", "ADV", "DET", "NOUN", "AUX", "ADV", "ADV", "PUNCT", "ADV", "PRON", "AUX", "VERB", "VERB", "ADP", "DET", "NOUN", "PUNCT", "ADV", "DET", "NOUN", "VERB", "PART", "VERB", "CCONJ", "DET", "NOUN", "CCONJ", "NOUN", "PART", "VERB", "ADV", "ADP", "DET", "NOUN", "PUNCT", "CCONJ", "SCONJ", "PRON", "AUX", "VERB", "ADV", "PART", "VERB", "DET", "NOUN", "ADP", "NOUN", "DET", "PROPN", "AUX", "VERB", "PRON", "NOUN", "ADP", "DET", "NOUN", "ADP", "NOUN"]),
    // stevenson p480s3
    ("stevenson", "p480s3",
     &["This", "even", "tint", "was", "indeed", "broken", "up", "by", "streaks", "of", "yellow", "sand-break", "in", "the", "lower", "lands", "and", "by", "many", "tall", "trees", "of", "the", "pine", "family", "out-topping", "the", "others", "--", "some", "singly", "some", "in", "clumps", ";", "but", "the", "general", "colouring", "was", "uniform", "and", "sad"],
     &["DET", "ADJ", "NOUN", "AUX", "ADV", "VERB", "ADV", "ADP", "NOUN", "ADP", "ADJ", "NOUN", "ADP", "DET", "ADJ", "NOUN", "CCONJ", "ADP", "ADJ", "ADJ", "NOUN", "ADP", "DET", "NOUN", "NOUN", "VERB", "DET", "NOUN", "PUNCT", "PRON", "ADV", "PRON", "ADP", "NOUN", "PUNCT", "CCONJ", "DET", "ADJ", "NOUN", "AUX", "ADJ", "CCONJ", "ADJ"]),
    // stevenson p513s0
    ("stevenson", "p513s0",
     &["All", "at", "once", "there", "began", "to", "go", "a", "sort", "of", "bustle", "among", "the", "bulrushes", ";", "a", "wild", "duck", "flew", "up", "with", "a", "quack", "another", "followed", "and", "soon", "over", "the", "whole", "surface", "of", "the", "marsh", "a", "great", "cloud", "of", "birds", "hung", "screaming", "and", "circling", "in", "the", "air"],
     &["DET", "ADP", "ADV", "PRON", "VERB", "PART", "VERB", "DET", "NOUN", "ADP", "NOUN", "ADP", "DET", "NOUN", "PUNCT", "DET", "ADJ", "NOUN", "VERB", "ADV", "ADP", "DET", "NOUN", "PRON", "VERB", "CCONJ", "ADV", "ADP", "DET", "ADJ", "NOUN", "ADP", "DET", "NOUN", "DET", "ADJ", "NOUN", "ADP", "NOUN", "VERB", "VERB", "CCONJ", "VERB", "ADP", "DET", "NOUN"]),
    // stevenson p1055s2
    ("stevenson", "p1055s2",
     &["I", "could", "only", "judge", "that", "all", "had", "perished", "and", "my", "heart", "smote", "me", "sorely", "that", "I", "had", "not", "been", "there", "to", "perish", "with", "them"],
     &["PRON", "AUX", "ADV", "VERB", "SCONJ", "PRON", "AUX", "VERB", "CCONJ", "PRON", "NOUN", "VERB", "PRON", "ADV", "SCONJ", "PRON", "AUX", "PART", "AUX", "ADV", "PART", "VERB", "ADP", "PRON"]),
    // stevenson p1084s5
    ("stevenson", "p1084s5",
     &["Cross", "me", "and", "you", "'ll", "go", "where", "many", "a", "good", "man", "'s", "gone", "before", "you", "first", "and", "last", "these", "thirty", "year", "back", "--", "some", "to", "the", "yard-arm", "shiver", "my", "timbers", "and", "some", "by", "the", "board", "and", "all", "to", "feed", "the", "fishes"],
     &["VERB", "PRON", "CCONJ", "PRON", "AUX", "VERB", "ADV", "DET", "DET", "ADJ", "NOUN", "AUX", "VERB", "ADP", "PRON", "ADJ", "CCONJ", "ADJ", "DET", "ADJ", "NOUN", "ADV", "PUNCT", "PRON", "ADP", "DET", "NOUN", "VERB", "PRON", "NOUN", "CCONJ", "PRON", "ADP", "DET", "NOUN", "CCONJ", "DET", "PART", "VERB", "DET", "NOUN"]),
];
