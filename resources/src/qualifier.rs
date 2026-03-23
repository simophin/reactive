use unic_langid::LanguageIdentifier;

/// Screen density, ordered from lowest to highest DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Density {
    Ldpi,
    Mdpi,
    Hdpi,
    Xhdpi,
    Xxhdpi,
    Xxxhdpi,
}

impl Density {
    fn rank(self) -> i32 {
        match self {
            Density::Ldpi => 0,
            Density::Mdpi => 1,
            Density::Hdpi => 2,
            Density::Xhdpi => 3,
            Density::Xxhdpi => 4,
            Density::Xxxhdpi => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    Light,
    Dark,
}

/// The set of qualifiers parsed from a resource directory name.
/// `None` on a field means the directory applies regardless of that dimension.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Copy)]
pub struct QualifierSet {
    pub locale: Option<&'static str>,
    pub density: Option<Density>,
    pub color_scheme: Option<ColorScheme>,
}

/// The current resource context used to select assets and format strings.
///
/// Store this in a signal (via [`crate::reactive::provide_resource_context`])
/// so the UI reacts when e.g. the user switches language, the window moves to
/// a different display, or the system dark-mode toggle fires.
///
/// Not `Copy` because [`LanguageIdentifier`] is heap-allocated.
#[derive(Debug, Clone)]
pub struct ResourceContext {
    pub locale: LanguageIdentifier,
    pub density: Density,
    pub color_scheme: ColorScheme,
}

impl Default for ResourceContext {
    fn default() -> Self {
        Self {
            locale: LanguageIdentifier::default(),
            density: Density::Mdpi,
            color_scheme: ColorScheme::Light,
        }
    }
}

/// Score a qualifier set against a resource context.
/// Returns `None` if the set is incompatible (e.g. specifies `night` but the
/// current scheme is `Light`, or specifies a different language).
/// Higher score = better match.
pub(crate) fn score(qualifiers: &QualifierSet, ctx: &ResourceContext) -> Option<i32> {
    let mut s = 0;

    if let Some(locale_str) = qualifiers.locale {
        let q_locale: LanguageIdentifier = locale_str.parse().ok()?;
        // Different language is a hard reject.
        if q_locale.language != ctx.locale.language {
            return None;
        }
        if q_locale == ctx.locale {
            s += 20; // exact match
        } else if q_locale.region == ctx.locale.region {
            s += 10; // same language + region (script differs)
        } else {
            s += 5; // same language only
        }
    }

    match qualifiers.color_scheme {
        Some(cs) if cs != ctx.color_scheme => return None,
        Some(_) => s += 10,
        None => {}
    }

    if let Some(d) = qualifiers.density {
        s += density_score(d, ctx.density);
    }

    Some(s)
}

/// Pick the best-matching value from a sequence of `(QualifierSet, T)` pairs.
///
/// Returns `None` only when the iterator is empty or every variant is
/// incompatible with `ctx`.
pub(crate) fn best_match<T>(
    candidates: impl Iterator<Item = (QualifierSet, T)>,
    ctx: &ResourceContext,
) -> Option<T> {
    candidates
        .filter_map(|(q, v)| score(&q, ctx).map(|s| (s, v)))
        .max_by_key(|(s, _)| *s)
        .map(|(_, v)| v)
}

fn density_score(available: Density, requested: Density) -> i32 {
    let a = available.rank();
    let r = requested.rank();
    if a == r {
        8
    } else if a > r {
        6 - (a - r) // prefer closest step up; won't lose quality
    } else {
        -(r - a) // upscaling required; last resort
    }
}
