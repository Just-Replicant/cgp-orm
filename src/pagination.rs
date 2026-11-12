//! Offset pagination request and page types.
//!
//! Page size is **never** hardcoded to 5. Contexts supply a
//! [`crate::components::HasPaginationPolicy`] with a default and a maximum;
//! [`PageRequest::resolve`] clamps the requested size and normalizes the
//! page index.
//!
//! # Cursor pagination (extension point)
//!
//! This module implements offset/limit pagination only. A future
//! `CursorRequest<Cursor> { after: Option<Cursor>, size: u64 }` together
//! with a `CanPaginateEntitiesByCursor` component can sit beside
//! [`PageRequest`] without changing this API. The ordering policy
//! ([`crate::components::CanBuildEntityOrder`]) is already required to be
//! deterministic, which is the prerequisite for keyset pagination.

/// A 1-based page request. Both fields are required; size is clamped by policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageRequest {
    /// 1-based page index. A value of `0` is treated as the first page.
    pub page: u64,
    /// Requested page size. A value of `0` selects the policy default.
    /// Values above the policy maximum are clamped.
    pub size: u64,
}

impl PageRequest {
    /// Build a request for `page` (1-based) and `size`.
    #[must_use]
    pub const fn new(page: u64, size: u64) -> Self {
        Self { page, size }
    }

    /// The first page, using the policy's default size (encoded as `size = 0`).
    #[must_use]
    pub const fn first() -> Self {
        Self { page: 1, size: 0 }
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
    /// Total number of rows matching the (unpaginated) query.
    pub total: u64,
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

    /// Whether there is a subsequent page.
    #[must_use]
    pub const fn has_next(&self) -> bool {
        self.page.saturating_mul(self.size) < self.total
    }

    /// Total number of pages given `size` (at least 1 if `size > 0`).
    #[must_use]
    pub const fn total_pages(&self) -> u64 {
        if self.size == 0 {
            0
        } else {
            self.total.div_ceil(self.size)
        }
    }
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::PageRequest;

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
}
