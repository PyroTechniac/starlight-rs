use crate::util::LitExt;
use proc_macro2::Span;
use std::fmt::{Display, Formatter, Result as FmtResult, Write};
use syn::{spanned::Spanned, Attribute, Error, Ident, Lit, LitStr, Meta, NestedMeta, Path, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
	Name,
	Equals,
	List,
	SingleList,
}

impl Display for ValueKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Name => f.pad("`#[<name>]`"),
			Self::Equals => f.pad("`#[<name> = <value>]`"),
			Self::List => f.pad("`#[<name>([<value>, <value>, <value>, ...])]`"),
			Self::SingleList => f.pad("`#[<name>(<value>)]`"),
		}
	}
}

fn to_ident(p: Path) -> Result<Ident> {
	if p.segments.is_empty() {
		return Err(Error::new(
			p.span(),
			"cannot convert an empty path to an identifier",
		));
	}

	if p.segments.len() > 1 {
		return Err(Error::new(
			p.span(),
			"the path must not have more than one segment",
		));
	}

	if !p.segments[0].arguments.is_empty() {
		return Err(Error::new(
			p.span(),
			"the singular path segment must not have any arguments",
		));
	}

	Ok(p.segments[0].ident.clone())
}

#[derive(Debug)]
pub struct Values {
	pub name: Ident,
	pub literals: Vec<Lit>,
	pub kind: ValueKind,
	pub span: Span,
}

impl Values {
	pub const fn new(name: Ident, kind: ValueKind, literals: Vec<Lit>, span: Span) -> Self {
		Self {
			name,
			literals,
			kind,
			span,
		}
	}
}

pub fn parse_values(attr: &Attribute) -> Result<Values> {
	let meta = attr.parse_meta()?;

	match meta {
		Meta::Path(path) => {
			let name = to_ident(path)?;

			Ok(Values::new(name, ValueKind::Name, vec![], attr.span()))
		}
		Meta::List(meta) => {
			let name = to_ident(meta.path)?;
			let nested = meta.nested;

			if nested.is_empty() {
				return Err(Error::new(attr.span(), "list cannot be empty"));
			}

			let mut lits = Vec::with_capacity(nested.len());

			for meta in nested {
				match meta {
                    NestedMeta::Lit(l) => lits.push(l),
                    NestedMeta::Meta(m) => match m {
                        Meta::Path(path) => {
                            let i = to_ident(path)?;
                            lits.push(Lit::Str(LitStr::new(&i.to_string(), i.span())))
                        }
                        Meta::List(_) | Meta::NameValue(_) => {
                            return Err(Error::new(attr.span(), "cannot nest a list; only accept literals and identifiers at this level"))
                        }
                    },
                }
			}

			let kind = if lits.len() == 1 {
				ValueKind::SingleList
			} else {
				ValueKind::List
			};

			Ok(Values::new(name, kind, lits, attr.span()))
		}
		Meta::NameValue(meta) => {
			let name = to_ident(meta.path)?;
			let lit = meta.lit;

			Ok(Values::new(name, ValueKind::Equals, vec![lit], attr.span()))
		}
	}
}

#[derive(Debug, Clone)]
struct DisplaySlice<'a, T>(&'a [T]);

impl<'a, T: Display> Display for DisplaySlice<'a, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut iter = self.0.iter().enumerate();

		match iter.next() {
			None => f.write_str("nothing")?,
			Some((idx, elem)) => {
				Display::fmt(&idx, f)?;
				f.write_str(": ")?;
				Display::fmt(&elem, f)?;

				for (idx, elem) in iter {
					f.write_char('\n')?;
					Display::fmt(&idx, f)?;
					f.write_str(": ")?;
					Display::fmt(&elem, f)?;
				}
			}
		}

		Ok(())
	}
}

#[inline]
fn is_form_acceptable(expect: &[ValueKind], kind: ValueKind) -> bool {
	if expect.contains(&ValueKind::List) && kind == ValueKind::SingleList {
		true
	} else {
		expect.contains(&kind)
	}
}

#[inline]
fn validate(values: &Values, forms: &[ValueKind]) -> Result<()> {
	if !is_form_acceptable(forms, values.kind) {
		return Err(Error::new(
			values.span,
			format_args!(
				"the attribute must be in one of these forms:\n{}",
				DisplaySlice(forms)
			),
		));
	}

	Ok(())
}

#[inline]
pub fn parse<T: AttributeOption>(values: Values) -> Result<T> {
	T::parse(values)
}

pub trait AttributeOption: Sized {
	fn parse(values: Values) -> Result<Self>;
}

impl AttributeOption for Vec<String> {
	fn parse(values: Values) -> Result<Self> {
		validate(&values, &[ValueKind::List])?;

		Ok(values
			.literals
			.into_iter()
			.map(|lit| lit.to_str())
			.collect())
	}
}

impl AttributeOption for String {
	fn parse(values: Values) -> Result<Self> {
		validate(&values, &[ValueKind::Equals, ValueKind::SingleList])?;

		Ok(values.literals[0].to_str())
	}
}

impl AttributeOption for bool {
	fn parse(values: Values) -> Result<Self> {
		validate(&values, &[ValueKind::Name, ValueKind::SingleList])?;

		Ok(values.literals.get(0).map_or(true, |l| l.to_bool()))
	}
}

impl AttributeOption for usize {
	fn parse(values: Values) -> Result<Self> {
		validate(
			&values,
			&[ValueKind::Name, ValueKind::Equals, ValueKind::SingleList],
		)?;

		Ok(values.literals[0].to_int())
	}
}
