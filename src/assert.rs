//! [`Output`][Output] assertions.
//!
//! [Output]: https://doc.rust-lang.org/std/process/struct.Output.html

use std::fmt;
use std::process;
use std::str;

use predicates;
use predicates::str::PredicateStrExt;
use predicates_core;
use predicates_tree::CaseTreeExt;

use errors::dump_buffer;
use errors::output_fmt;

/// Assert the state of an [`Output`].
///
/// # Examples
///
/// ```rust
/// use assert_cmd::prelude::*;
///
/// use std::process::Command;
///
/// Command::main_binary()
///     .unwrap()
///     .assert()
///     .success();
/// ```
///
/// [`Output`]: https://doc.rust-lang.org/std/process/struct.Output.html
pub trait OutputAssertExt {
    /// Wrap with an interface for that provides assertions on the [`Output`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .assert()
    ///     .success();
    /// ```
    ///
    /// [`Output`]: https://doc.rust-lang.org/std/process/struct.Output.html
    fn assert(self) -> Assert;
}

impl OutputAssertExt for process::Output {
    fn assert(self) -> Assert {
        Assert::new(self)
    }
}

impl<'c> OutputAssertExt for &'c mut process::Command {
    fn assert(self) -> Assert {
        let output = self.output().unwrap();
        Assert::new(output).append_context("command", format!("{:?}", self))
    }
}

/// Assert the state of an [`Output`].
///
/// Create an `Assert` through the [`OutputAssertExt`] trait.
///
/// # Examples
///
/// ```rust
/// use assert_cmd::prelude::*;
///
/// use std::process::Command;
///
/// Command::main_binary()
///     .unwrap()
///     .assert()
///     .success();
/// ```
///
/// [`Output`]: https://doc.rust-lang.org/std/process/struct.Output.html
/// [`OutputAssertExt`]: trait.OutputAssertExt.html
pub struct Assert {
    output: process::Output,
    context: Vec<(&'static str, Box<fmt::Display>)>,
}

impl Assert {
    /// Create an `Assert` for a given [`Output`].
    ///
    /// [`Output`]: https://doc.rust-lang.org/std/process/struct.Output.html
    pub fn new(output: process::Output) -> Self {
        Self {
            output,
            context: vec![],
        }
    }

    /// Clarify failures with additional context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .assert()
    ///     .append_context("main", "no args")
    ///     .success();
    /// ```
    pub fn append_context<D>(mut self, name: &'static str, context: D) -> Self
    where
        D: fmt::Display + 'static,
    {
        self.context.push((name, Box::new(context)));
        self
    }

    /// Access the contained [`Output`].
    ///
    /// [`Output`]: https://doc.rust-lang.org/std/process/struct.Output.html
    pub fn get_output(&self) -> &process::Output {
        &self.output
    }

    /// Ensure the command succeeded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .assert()
    ///     .success();
    /// ```
    pub fn success(self) -> Self {
        if !self.output.status.success() {
            let actual_code = self.output.status.code().unwrap_or_else(|| {
                panic!(
                    "Unexpected failure.\ncode=<interrupted>\nstderr=```{}```\n{}",
                    dump_buffer(&self.output.stderr),
                    self
                )
            });
            panic!(
                "Unexpected failure.\ncode-{}\nstderr=```{}```\n{}",
                actual_code,
                dump_buffer(&self.output.stderr),
                self
            );
        }
        self
    }

    /// Ensure the command failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("exit", "1")
    ///     .assert()
    ///     .failure();
    /// ```
    pub fn failure(self) -> Self {
        if self.output.status.success() {
            panic!("Unexpected success\n{}", self);
        }
        self
    }

    /// Ensure the command aborted before returning a code.
    pub fn interrupted(self) -> Self {
        if self.output.status.code().is_some() {
            panic!("Unexpected completion\n{}", self);
        }
        self
    }

    /// Ensure the command returned the expected code.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    /// use predicates::prelude::*;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("exit", "42")
    ///     .assert()
    ///     .code(predicate::eq(42));
    ///
    /// // which can be shortened to:
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("exit", "42")
    ///     .assert()
    ///     .code(42);
    /// ```
    ///
    /// - See [`predicates::prelude`] for more predicates.
    /// - See [`IntoCodePredicate`] for other built-in conversions.
    ///
    /// [`predicates::prelude`]: https://docs.rs/predicates/0.9.0/predicates/prelude/
    /// [`IntoCodePredicate`]: trait.IntoCodePredicate.html
    pub fn code<I, P>(self, pred: I) -> Self
    where
        I: IntoCodePredicate<P>,
        P: predicates_core::Predicate<i32>,
    {
        self.code_impl(&pred.into_code())
    }

    fn code_impl(self, pred: &predicates_core::Predicate<i32>) -> Self {
        let actual_code = self.output
            .status
            .code()
            .unwrap_or_else(|| panic!("Command interrupted\n{}", self));
        if let Some(case) = pred.find_case(false, &actual_code) {
            panic!("Unexpected return code, failed {}\n{}", case.tree(), self);
        }
        self
    }

    /// Ensure the command wrote the expected data to `stdout`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    /// use predicates::prelude::*;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("stdout", "hello")
    ///     .env("stderr", "world")
    ///     .assert()
    ///     .stdout(predicate::str::similar("hello\n").from_utf8());
    ///
    /// // which can be shortened to:
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("stdout", "hello")
    ///     .env("stderr", "world")
    ///     .assert()
    ///     .stdout("hello\n");
    /// ```
    ///
    /// - See [`predicates::prelude`] for more predicates.
    /// - See [`IntoOutputPredicate`] for other built-in conversions.
    ///
    /// [`predicates::prelude`]: https://docs.rs/predicates/0.9.0/predicates/prelude/
    /// [`IntoOutputPredicate`]: trait.IntoOutputPredicate.html
    pub fn stdout<I, P>(self, pred: I) -> Self
    where
        I: IntoOutputPredicate<P>,
        P: predicates_core::Predicate<[u8]>,
    {
        self.stdout_impl(&pred.into_output())
    }

    fn stdout_impl(self, pred: &predicates_core::Predicate<[u8]>) -> Self {
        {
            let actual = &self.output.stdout;
            if let Some(case) = pred.find_case(false, &actual) {
                panic!("Unexpected stdout, failed {}\n{}", case.tree(), self);
            }
        }
        self
    }

    /// Ensure the command wrote the expected data to `stderr`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use assert_cmd::prelude::*;
    ///
    /// use std::process::Command;
    /// use predicates::prelude::*;
    ///
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("stdout", "hello")
    ///     .env("stderr", "world")
    ///     .assert()
    ///     .stderr(predicate::str::similar("world\n").from_utf8());
    ///
    /// // which can be shortened to:
    /// Command::main_binary()
    ///     .unwrap()
    ///     .env("stdout", "hello")
    ///     .env("stderr", "world")
    ///     .assert()
    ///     .stderr("world\n");
    /// ```
    ///
    /// - See [`predicates::prelude`] for more predicates.
    /// - See [`IntoOutputPredicate`] for other built-in conversions.
    ///
    /// [`predicates::prelude`]: https://docs.rs/predicates/0.9.0/predicates/prelude/
    /// [`IntoOutputPredicate`]: trait.IntoOutputPredicate.html
    pub fn stderr<I, P>(self, pred: I) -> Self
    where
        I: IntoOutputPredicate<P>,
        P: predicates_core::Predicate<[u8]>,
    {
        self.stderr_impl(&pred.into_output())
    }

    fn stderr_impl(self, pred: &predicates_core::Predicate<[u8]>) -> Self {
        {
            let actual = &self.output.stderr;
            if let Some(case) = pred.find_case(false, &actual) {
                panic!("Unexpected stderr, failed {}\n\n{}", case.tree(), self);
            }
        }
        self
    }
}

impl fmt::Display for Assert {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &(ref name, ref context) in &self.context {
            writeln!(f, "{}=`{}`", name, context)?;
        }
        output_fmt(&self.output, f)
    }
}

impl fmt::Debug for Assert {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Assert")
            .field("output", &self.output)
            .finish()
    }
}

/// Used by [`Assert::code`] to convert `Self` into the needed
/// [`Predicate<i32>`].
///
/// # Examples
///
/// ```rust,ignore
/// use assert_cmd::prelude::*;
///
/// use std::process::Command;
/// use predicates::prelude::*;
///
/// Command::main_binary()
///     .unwrap()
///     .env("exit", "42")
///     .assert()
///     .code(predicate::eq(42));
///
/// // which can be shortened to:
/// Command::main_binary()
///     .unwrap()
///     .env("exit", "42")
///     .assert()
///     .code(42);
/// ```
///
/// [`Assert::code`]: struct.Assert.html#method.code
/// [`Predicate<i32>`]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
pub trait IntoCodePredicate<P>
where
    P: predicates_core::Predicate<i32>,
{
    /// The type of the predicate being returned.
    type Predicate;

    /// Convert to a predicate for testing a program's exit code.
    fn into_code(self) -> P;
}

impl<P> IntoCodePredicate<P> for P
where
    P: predicates_core::Predicate<i32>,
{
    type Predicate = P;

    fn into_code(self) -> Self::Predicate {
        self
    }
}

// Keep `predicates` concrete Predicates out of our public API.
/// [Predicate] used by [`IntoCodePredicate`] for bytes.
///
/// [`IntoCodePredicate`]: trait.IntoCodePredicate.html
/// [Predicate]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
#[derive(Debug)]
pub struct EqCodePredicate(predicates::ord::EqPredicate<i32>);

impl EqCodePredicate {
    pub(crate) fn new(value: i32) -> Self {
        let pred = predicates::ord::eq(value);
        EqCodePredicate(pred)
    }
}

impl predicates_core::reflection::PredicateReflection for EqCodePredicate {
    fn parameters<'a>(
        &'a self,
    ) -> Box<Iterator<Item = predicates_core::reflection::Parameter<'a>> + 'a> {
        self.0.parameters()
    }

    /// Nested `Predicate`s of the current `Predicate`.
    fn children<'a>(&'a self) -> Box<Iterator<Item = predicates_core::reflection::Child<'a>> + 'a> {
        self.0.children()
    }
}

impl predicates_core::Predicate<i32> for EqCodePredicate {
    fn eval(&self, item: &i32) -> bool {
        self.0.eval(item)
    }

    fn find_case<'a>(
        &'a self,
        expected: bool,
        variable: &i32,
    ) -> Option<predicates_core::reflection::Case<'a>> {
        self.0.find_case(expected, variable)
    }
}

impl fmt::Display for EqCodePredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoCodePredicate<EqCodePredicate> for i32 {
    type Predicate = EqCodePredicate;

    fn into_code(self) -> Self::Predicate {
        Self::Predicate::new(self)
    }
}

// Keep `predicates` concrete Predicates out of our public API.
/// [Predicate] used by [`IntoCodePredicate`] for bytes.
///
/// [`IntoCodePredicate`]: trait.IntoCodePredicate.html
/// [Predicate]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
#[derive(Debug)]
pub struct InCodePredicate(predicates::iter::InPredicate<i32>);

impl InCodePredicate {
    pub(crate) fn new<I:IntoIterator<Item=i32>>(value: I) -> Self {
        let pred = predicates::iter::in_iter(value);
        InCodePredicate(pred)
    }
}

impl predicates_core::reflection::PredicateReflection for InCodePredicate {
    fn parameters<'a>(
        &'a self,
    ) -> Box<Iterator<Item = predicates_core::reflection::Parameter<'a>> + 'a> {
        self.0.parameters()
    }

    /// Nested `Predicate`s of the current `Predicate`.
    fn children<'a>(&'a self) -> Box<Iterator<Item = predicates_core::reflection::Child<'a>> + 'a> {
        self.0.children()
    }
}

impl predicates_core::Predicate<i32> for InCodePredicate {
    fn eval(&self, item: &i32) -> bool {
        self.0.eval(item)
    }

    fn find_case<'a>(
        &'a self,
        expected: bool,
        variable: &i32,
    ) -> Option<predicates_core::reflection::Case<'a>> {
        self.0.find_case(expected, variable)
    }
}

impl fmt::Display for InCodePredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoCodePredicate<InCodePredicate> for Vec<i32> {
    type Predicate = InCodePredicate;

    fn into_code(self) -> Self::Predicate {
        Self::Predicate::new(self)
    }
}

impl IntoCodePredicate<InCodePredicate> for &'static [i32] {
    type Predicate = InCodePredicate;

    fn into_code(self) -> Self::Predicate {
        Self::Predicate::new(self.iter().cloned())
    }
}

/// Used by [`Assert::stdout`] and [`Assert::stderr`] to convert Self
/// into the needed [`Predicate<[u8]>`].
///
/// # Examples
///
/// ```rust,ignore
/// use assert_cmd::prelude::*;
///
/// use std::process::Command;
/// use predicates::prelude::*;
///
/// Command::main_binary()
///     .unwrap()
///     .env("stdout", "hello")
///     .env("stderr", "world")
///     .assert()
///     .stdout(predicate::str::similar("hello\n").from_utf8());
///
/// // which can be shortened to:
/// Command::main_binary()
///     .unwrap()
///     .env("stdout", "hello")
///     .env("stderr", "world")
///     .assert()
///     .stdout("hello\n");
/// ```
///
/// [`Assert::stdout`]: struct.Assert.html#method.stdout
/// [`Assert::stderr`]: struct.Assert.html#method.stderr
/// [`Predicate<[u8]>`]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
pub trait IntoOutputPredicate<P>
where
    P: predicates_core::Predicate<[u8]>,
{
    /// The type of the predicate being returned.
    type Predicate;

    /// Convert to a predicate for testing a path.
    fn into_output(self) -> P;
}

impl<P> IntoOutputPredicate<P> for P
where
    P: predicates_core::Predicate<[u8]>,
{
    type Predicate = P;

    fn into_output(self) -> Self::Predicate {
        self
    }
}

// Keep `predicates` concrete Predicates out of our public API.
/// [Predicate] used by [`IntoOutputPredicate`] for bytes.
///
/// [`IntoOutputPredicate`]: trait.IntoOutputPredicate.html
/// [Predicate]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
#[derive(Debug)]
pub struct BytesContentOutputPredicate(predicates::ord::EqPredicate<&'static [u8]>);

impl BytesContentOutputPredicate {
    pub(crate) fn new(value: &'static [u8]) -> Self {
        let pred = predicates::ord::eq(value);
        BytesContentOutputPredicate(pred)
    }
}

impl predicates_core::reflection::PredicateReflection for BytesContentOutputPredicate {
    fn parameters<'a>(
        &'a self,
    ) -> Box<Iterator<Item = predicates_core::reflection::Parameter<'a>> + 'a> {
        self.0.parameters()
    }

    /// Nested `Predicate`s of the current `Predicate`.
    fn children<'a>(&'a self) -> Box<Iterator<Item = predicates_core::reflection::Child<'a>> + 'a> {
        self.0.children()
    }
}

impl predicates_core::Predicate<[u8]> for BytesContentOutputPredicate {
    fn eval(&self, item: &[u8]) -> bool {
        self.0.eval(item)
    }

    fn find_case<'a>(
        &'a self,
        expected: bool,
        variable: &[u8],
    ) -> Option<predicates_core::reflection::Case<'a>> {
        self.0.find_case(expected, variable)
    }
}

impl fmt::Display for BytesContentOutputPredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoOutputPredicate<BytesContentOutputPredicate> for &'static [u8] {
    type Predicate = BytesContentOutputPredicate;

    fn into_output(self) -> Self::Predicate {
        Self::Predicate::new(self)
    }
}

// Keep `predicates` concrete Predicates out of our public API.
/// [Predicate] used by [`IntoOutputPredicate`] for [`str`].
///
/// [`IntoOutputPredicate`]: trait.IntoOutputPredicate.html
/// [Predicate]: https://docs.rs/predicates-core/0.9.0/predicates_core/trait.Predicate.html
/// [`str`]: https://doc.rust-lang.org/std/primitive.str.html
#[derive(Debug, Clone)]
pub struct StrContentOutputPredicate(
    predicates::str::Utf8Predicate<predicates::str::DifferencePredicate>,
);

impl StrContentOutputPredicate {
    pub(crate) fn new(value: &'static str) -> Self {
        let pred = predicates::str::similar(value).from_utf8();
        StrContentOutputPredicate(pred)
    }
}

impl predicates_core::reflection::PredicateReflection for StrContentOutputPredicate {
    fn parameters<'a>(
        &'a self,
    ) -> Box<Iterator<Item = predicates_core::reflection::Parameter<'a>> + 'a> {
        self.0.parameters()
    }

    /// Nested `Predicate`s of the current `Predicate`.
    fn children<'a>(&'a self) -> Box<Iterator<Item = predicates_core::reflection::Child<'a>> + 'a> {
        self.0.children()
    }
}

impl predicates_core::Predicate<[u8]> for StrContentOutputPredicate {
    fn eval(&self, item: &[u8]) -> bool {
        self.0.eval(item)
    }

    fn find_case<'a>(
        &'a self,
        expected: bool,
        variable: &[u8],
    ) -> Option<predicates_core::reflection::Case<'a>> {
        self.0.find_case(expected, variable)
    }
}

impl fmt::Display for StrContentOutputPredicate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoOutputPredicate<StrContentOutputPredicate> for &'static str {
    type Predicate = StrContentOutputPredicate;

    fn into_output(self) -> Self::Predicate {
        Self::Predicate::new(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use predicates::prelude::*;

    // Since IntoCodePredicate exists solely for conversion, test it under that scenario to ensure
    // it works as expected.
    fn convert_code<I, P>(pred: I) -> P
    where
        I: IntoCodePredicate<P>,
        P: predicates_core::Predicate<i32>,
    {
        pred.into_code()
    }

    #[test]
    fn into_code_from_pred() {
        let pred = convert_code(predicate::eq(10));
        assert!(pred.eval(&10));
    }

    #[test]
    fn into_code_from_i32() {
        let pred = convert_code(10);
        assert!(pred.eval(&10));
    }

    #[test]
    fn into_code_from_vec() {
        let pred = convert_code(vec![3, 10]);
        assert!(pred.eval(&10));
    }

    #[test]
    fn into_code_from_array() {
        let pred = convert_code(&[3, 10] as &[i32]);
        assert!(pred.eval(&10));
    }

    // Since IntoOutputPredicate exists solely for conversion, test it under that scenario to ensure
    // it works as expected.
    fn convert_output<I, P>(pred: I) -> P
    where
        I: IntoOutputPredicate<P>,
        P: predicates_core::Predicate<[u8]>,
    {
        pred.into_output()
    }

    #[test]
    fn into_output_from_pred() {
        let pred = convert_output(predicate::eq(b"Hello" as &[u8]));
        assert!(pred.eval(b"Hello" as &[u8]));
    }

    #[test]
    fn into_output_from_bytes() {
        let pred = convert_output(b"Hello" as &[u8]);
        assert!(pred.eval(b"Hello" as &[u8]));
    }

    #[test]
    fn into_output_from_str() {
        let pred = convert_output("Hello");
        assert!(pred.eval(b"Hello" as &[u8]));
    }
}
