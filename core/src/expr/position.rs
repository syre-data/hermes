#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_more::Deref, derive_more::From)]
pub struct BytePos(pub usize);

impl std::ops::Add<usize> for BytePos {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub fn new(start: impl Into<BytePos>, end: impl Into<BytePos>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Span a single position.
    pub fn at(position: impl Into<BytePos>) -> Self {
        let pos = position.into();
        Self {
            start: pos,
            end: pos + 1,
        }
    }
}

#[derive(Debug)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: Span,
}

impl<T> WithSpan<T> {
    pub fn new(value: T, start: impl Into<BytePos>, end: impl Into<BytePos>) -> Self {
        Self {
            value,
            span: Span::new(start, end),
        }
    }

    /// Span a single position.
    pub fn at(value: T, pos: impl Into<BytePos>) -> Self {
        Self {
            value,
            span: Span::at(pos),
        }
    }
}
