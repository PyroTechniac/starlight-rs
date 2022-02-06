use twilight_model::application::interaction::application_command::CommandData;
use twilight_util::builder::command::CommandBuilder;
use futures_util::future::ok;

use super::SlashData;
use crate::{helpers::InteractionsHelper, prelude::*};

pub trait SlashCommand: Send + Sync {
	fn run<'a>(
		&'a self,
		helper: InteractionsHelper,
		responder: SlashData,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

	#[allow(unused_variables)]
	fn autocomplete<'a>(
		&'a self,
		helper: InteractionsHelper,
		responder: SlashData,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
		ok(()).boxed()
	}
}

pub trait DefineCommand: SlashCommand + Sized {
	fn define() -> CommandBuilder;

	fn parse(data: CommandData) -> Result<Self>;
}
