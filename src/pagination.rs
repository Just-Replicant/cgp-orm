//! Offset pagination request and page types.
//!
//! Page size is **never** hardcoded to 5. Contexts supply a
//! [`crate::components::HasPaginationPolicy`] with a default and a maximum;
//! [`PageRequest::resolve`] clamps the requested size and normalizes the
//! page index.
//!
//! The repository applies [`crate::components::CanBuildEntityFilter`] to
//! both the page query and the `COUNT(*)`. To skip the count, use
//! [`PageRequest::without_total`]; [`Page::total`] is then `None`.
//!
//! # Cursor pagination (extension point)
//!
//! This module implements offset/limit pagination only. A future
//! `CursorRequest<Cursor> { after: Option<Cursor>, size: u64 }` together
//! with a `CanPaginateEntitiesByCursor` component can sit beside
//! [`PageRequest`] without changing this API. The ordering policy
//! ([`crate::components::CanBuildEntityOrder`]) is already required to be
//! deterministic, which is the prerequisite for keyset pagination.
//! Cursor pagination is the milestone after the entity filter, not a
//! substitute for it, and is not implemented here.

/// A 1-based page request. Page and size are required; size is clamped by policy.
///
/// By default the repository also runs `COUNT(*)` and fills [`Page::total`].
/// Call [`PageRequest::without_total`] to skip that query; `total` is then
/// `None`. Existing callers of [`PageRequest::new`] and [`PageRequest::first`]
/// keep the count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    /// 1-based page index. A value of `0` is treated as the first page.
    pub page: u64,
    /// Requested page size. A value of `0` selects the policy default.
    /// Values above the policy maximum are clamped.
    pub size: u64,
    /// When `true` (the default), the repository runs `COUNT(*)` and
    /// fills [`Page::total`]. When `false`, that query is skipped and
    /// `total` is `None`.
    pub include_total: bool,
}

impl PageRequest {
    /// Build a request for `page` (1-based) and `size`, including a total count.
    #[must_use]
    pub const fn new(page: u64, size: u64) -> Self {
        Self {
            page,
            size,
            include_total: true,
        }
    }

    /// The first page, using the policy's default size (encoded as `size = 0`),
    /// including a total count.
    #[must_use]
    pub const fn first() -> Self {
        Self {
            page: 1,
            size: 0,
            include_total: true,
        }
    }

    /// Same request, but skip `COUNT(*)`. [`Page::total`] will be `None`.
    #[must_use]
    pub const fn without_total(mut self) -> Self {
        self.include_total = false;
        self
    }

    /// Explicitly include (`true`) or skip (`false`) the total count.
    #[must_use]
    pub const fn with_total(mut self, include_total: bool) -> Self {
        self.include_total = include_total;
        self
    }

    /// Apply a pagination policy: clamp size, normalize the page, compute
    /// the offset.
    ///
    /// * `page == 0` becomes page `1`.
    /// * `size == 0` becomes `default_size`, itself clamped into
    ///   `[1, max_size]`.
    /// * A positive `size` is clamped into `[1, max_size]`.
    #[must_use]
    pub fn resolve(&self, default_size: u64, max_size: u64) -> ResolvedPage {
        let max_size = max_size.max(1);
        let default_size = default_size.clamp(1, max_size);
        let size = if self.size == 0 {
            default_size
        } else {
            self.size.clamp(1, max_size)
        };
        let page = self.page.max(1);
        ResolvedPage {
            page,
            size,
            offset: page.saturating_sub(1).saturating_mul(size),
            include_total: self.include_total,
        }
    }
}

/// A [`PageRequest`] after a pagination policy has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedPage {
    /// 1-based, at least 1.
    pub page: u64,
    /// Clamped page size, at least 1 and at most the policy maximum.
    pub size: u64,
    /// ` (page - 1) * size `, for backends that want `LIMIT/OFFSET` directly.
    pub offset: u64,
    /// Whether the repository should run `COUNT(*)` and fill [`Page::total`].
    pub include_total: bool,
}

impl ResolvedPage {
    /// 0-based page index for SeaORM's [`PaginatorTrait::fetch_page`].
    #[must_use]
    pub const fn zero_based_page(&self) -> u64 {
        self.page.saturating_sub(1)
    }
}

/// A page of items plus the pagination metadata used to fetch it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    /// Items in this page, in the repository's deterministic order.
    pub items: Vec<T>,
    /// 1-based page that was fetched (after policy normalization).
    pub page: u64,
    /// Page size that was applied (after clamping).
    pub size: u64,
    /// Total number of rows matching the (unpaginated) query, including
    /// the entity filter.
    ///
    /// `None` when the request skipped `COUNT(*)` via
    /// [`PageRequest::without_total`].
    pub total: Option<u64>,
}

impl<T> Page<T> {
    /// Map each item, keeping pagination metadata.
    pub fn map<U, F>(self, f: F) -> Page<U>
    where
        F: FnMut(T) -> U,
    {
        Page {
            items: self.items.into_iter().map(f).collect(),
            page: self.page,
            size: self.size,
            total: self.total,
        }
    }

    /// Map each item with a fallible function.
    pub fn try_map<U, E, F>(self, f: F) -> Result<Page<U>, E>
    where
        F: FnMut(T) -> Result<U, E>,
    {
        Ok(Page {
            items: self
                .items
                .into_iter()
                .map(f)
                .collect::<Result<Vec<_>, E>>()?,
            page: self.page,
            size: self.size,
            total: self.total,
        })
    }

    /// Whether there is a subsequent page, when a total is known.
    ///
    /// Returns `None` when the count was skipped ([`Page::total`] is
    /// `None`). Use [`Page::maybe_has_next`] for a full-page heuristic
    /// in that case.
    #[must_use]
    pub const fn has_next(&self) -> Option<bool> {
        match self.total {
            Some(total) => Some(self.page.saturating_mul(self.size) < total),
            None => None,
        }
    }

    /// Whether a subsequent page may exist.
    ///
    /// When [`Page::total`] is known this is the same as [`Page::has_next`].
    /// When the count was skipped, a full page of items is treated as a
    /// hint that another page might exist.
    #[must_use]
    pub fn maybe_has_next(&self) -> bool {
        match self.has_next() {
            Some(known) => known,
            None => (self.items.len() as u64) >= self.size,
        }
    }

    /// Total number of pages given `size`, when a total is known.
    ///
    /// Returns `None` when the count was skipped. When `size` is `0` and
    /// a total is present, this returns `Some(0)`.
    #[must_use]
    pub const fn total_pages(&self) -> Option<u64> {
        match self.total {
            None => None,
            Some(_) if self.size == 0 => Some(0),
            Some(total) => Some(total.div_ceil(self.size)),
        }
    }
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::{Page, PageRequest};

    #[test]
    fn zero_size_uses_default_clamped_to_max() {
        let resolved = PageRequest::new(1, 0).resolve(20, 100);
        assert_eq!(resolved.size, 20);
        assert_eq!(resolved.page, 1);
        assert_eq!(resolved.offset, 0);
    }

    #[test]
    fn size_is_clamped_to_max() {
        let resolved = PageRequest::new(2, 999).resolve(20, 7);
        assert_eq!(resolved.size, 7);
        assert_eq!(resolved.page, 2);
        assert_eq!(resolved.offset, 7);
    }

    #[test]
    fn page_zero_is_first_page() {
        let resolved = PageRequest::new(0, 10).resolve(20, 100);
        assert_eq!(resolved.page, 1);
        assert_eq!(resolved.offset, 0);
        assert_eq!(resolved.zero_based_page(), 0);
    }

    #[test]
    fn default_above_max_is_clamped() {
        let resolved = PageRequest::new(1, 0).resolve(50, 10);
        assert_eq!(resolved.size, 10);
    }

    #[test]
    fn never_hardcodes_five() {
        // A request that does not specify a size must not silently become 5
        // unless the policy says so. The crate default is 20 / 100.
        let resolved = PageRequest::first().resolve(20, 100);
        assert_ne!(resolved.size, 5);
        assert_eq!(resolved.size, 20);
    }

    #[test]
    fn new_and_first_include_the_total_by_default() {
        let resolved = PageRequest::new(1, 10).resolve(20, 100);
        assert!(resolved.include_total);
        let resolved = PageRequest::first().resolve(20, 100);
        assert!(resolved.include_total);
    }

    #[test]
    fn without_total_skips_the_count() {
        let request = PageRequest::new(2, 10).without_total();
        assert!(!request.include_total);
        let resolved = request.resolve(20, 100);
        assert!(!resolved.include_total);
        assert_eq!(resolved.page, 2);
        assert_eq!(resolved.size, 10);
    }

    #[test]
    fn has_next_and_total_pages_require_a_total() {
        let with_total = Page {
            items: vec![1, 2],
            page: 1,
            size: 2,
            total: Some(5),
        };
        assert_eq!(with_total.has_next(), Some(true));
        assert_eq!(with_total.total_pages(), Some(3));
        assert!(with_total.maybe_has_next());

        let last = Page {
            items: vec![5],
            page: 3,
            size: 2,
            total: Some(5),
        };
        assert_eq!(last.has_next(), Some(false));
        assert!(!last.maybe_has_next());

        let unknown_full = Page {
            items: vec![1, 2],
            page: 1,
            size: 2,
            total: None,
        };
        assert_eq!(unknown_full.has_next(), None);
        assert_eq!(unknown_full.total_pages(), None);
        assert!(unknown_full.maybe_has_next());

        let unknown_short = Page {
            items: vec![1],
            page: 1,
            size: 2,
            total: None,
        };
        assert!(!unknown_short.maybe_has_next());
    }
}
