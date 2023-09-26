use core::fmt;

pub(crate) trait Join<I, S> {
    fn join(self, sep: S) -> JoinIter<I, S>;
}

impl<I: Iterator, S> Join<I, S> for I
where
    S: fmt::Display,
{
    fn join(self, sep: S) -> JoinIter<I, S> {
        impl<I: Iterator, S: fmt::Display> fmt::Display for JoinIter<I, S>
        where
            I: Clone,
            I::Item: fmt::Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut values = self.inner.clone().peekable();
                while let Some(v) = values.next() {
                    write!(f, "{v}")?;

                    if values.peek().is_some() {
                        write!(f, "{}", self.sep)?;
                    }
                }
                Ok(())
            }
        }

        JoinIter { inner: self, sep }
    }
}

#[derive(Clone)]
pub(crate) struct JoinIter<I, S> {
    inner: I,
    sep: S,
}

pub(crate) trait Format<I, F> {
    fn format(self, formatter: F) -> FormatIter<I, F>;
}

impl<I: Iterator, F> Format<I, F> for I {
    fn format(self, formatter: F) -> FormatIter<I, F> {
        impl<T, F> fmt::Display for FormatItem<T, F>
        where
            F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.formatter)(&self.inner, f)
            }
        }

        impl<I: Iterator, F> Iterator for FormatIter<I, F>
        where
            F: Clone,
        {
            type Item = FormatItem<I::Item, F>;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map(|v| Self::Item {
                    inner: v,
                    formatter: self.formatter.clone(),
                })
            }
        }

        FormatIter {
            inner: self,
            formatter,
        }
    }
}

#[derive(Clone)]
pub(crate) struct FormatIter<I, F> {
    inner: I,
    formatter: F,
}

pub(crate) struct FormatItem<T, F> {
    inner: T,
    formatter: F,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_join() {
        let join = [0].iter().skip(1).join(", ");
        assert_eq!(format!("{join}"), "");

        let join = [1].iter().join(", ");
        assert_eq!(format!("{join}"), "1");

        let join = [2, 3].iter().join(", ");
        assert_eq!(format!("{join}"), "2, 3");
    }

    #[test]
    fn format() {
        let f = |v: &_, f: &mut fmt::Formatter<'_>| write!(f, "<{v}>");
        let fmt = [true, false].iter().format(f).map(|v| format!("{v}"));
        assert_eq!(fmt.collect::<Vec<_>>(), &["<true>", "<false>"]);
    }

    #[test]
    fn format_join() {
        let f = |v: &_, f: &mut fmt::Formatter<'_>| write!(f, "[[{v}]]");
        let fmt = ['a', 'b'].iter().format(f).join("_");
        assert_eq!(format!("{fmt}"), "[[a]]_[[b]]".to_string());
    }
}
