pub(super) const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Characters a name can't contain on top of control characters: every Unicode
/// format character (Cf, Unicode 16), the line and paragraph separators, and
/// the blank Hangul fillers. They're all invisible or reorder text. The Windows
/// app keeps the same list (`LocalControlConstants.InvisibleRanges`), and the
/// shared names.json fixture checks both.
pub(super) const INVISIBLE: [std::ops::RangeInclusive<char>; 24] = [
    '\u{00AD}'..='\u{00AD}',
    '\u{0600}'..='\u{0605}',
    '\u{061C}'..='\u{061C}',
    '\u{06DD}'..='\u{06DD}',
    '\u{070F}'..='\u{070F}',
    '\u{0890}'..='\u{0891}',
    '\u{08E2}'..='\u{08E2}',
    '\u{115F}'..='\u{1160}',
    '\u{180E}'..='\u{180E}',
    '\u{200B}'..='\u{200F}',
    '\u{2028}'..='\u{2029}',
    '\u{202A}'..='\u{202E}',
    '\u{2060}'..='\u{206F}',
    '\u{3164}'..='\u{3164}',
    '\u{FEFF}'..='\u{FEFF}',
    '\u{FFA0}'..='\u{FFA0}',
    '\u{FFF9}'..='\u{FFFB}',
    '\u{110BD}'..='\u{110BD}',
    '\u{110CD}'..='\u{110CD}',
    '\u{13430}'..='\u{1343F}',
    '\u{1BCA0}'..='\u{1BCA3}',
    '\u{1D173}'..='\u{1D17A}',
    '\u{E0001}'..='\u{E0001}',
    '\u{E0020}'..='\u{E007F}',
];
