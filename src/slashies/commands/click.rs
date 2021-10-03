use super::{ClickCommand, SlashCommand};
use crate::{
	components::{BuildError, ButtonBuilder, ComponentBuilder},
	slashies::{interaction::Interaction, Response},
	state::State,
	utils::interaction_author,
};
use async_trait::async_trait;
use miette::{IntoDiagnostic, Result};
use twilight_model::application::{
	command::{Command, CommandType},
	component::{button::ButtonStyle, Button},
	interaction::ApplicationCommand,
};

#[derive(Debug, Clone)]
pub struct Click(pub(super) ApplicationCommand);

#[async_trait]
impl SlashCommand<2> for Click {
	const NAME: &'static str = "click";

	fn define() -> Command {
		Command {
			application_id: None,
			default_permission: None,
			description: String::from("Sends a clickyboi"),
			guild_id: None,
			id: None,
			name: String::from(Self::NAME),
			options: vec![],
			kind: CommandType::ChatInput,
		}
	}

	async fn run(&self, state: State) -> Result<()> {
		let interaction = state.interaction(&self.0);

		let response = Response::new()
			.message("Click this")
			.add_components(Self::components().into_diagnostic()?);

		interaction.response(response).await.into_diagnostic()?;

		let click_data =
			Self::wait_for_click(state, interaction, interaction_author(interaction.command))
				.await?;

		interaction
			.update()
			.into_diagnostic()?
			.content(Some(
				format!(
					"Success! You clicked {}",
					Self::parse(interaction, &click_data.data.custom_id)
				)
				.as_str(),
			))
			.into_diagnostic()?
			.components(Self::EMPTY_COMPONENTS)
			.into_diagnostic()?
			.exec()
			.await
			.into_diagnostic()?;

		Ok(())
	}
}

#[async_trait]
impl ClickCommand<2> for Click {
	type Output = String;

	// const BUTTON_LABELS: [&'static str; 2] = ["A button", "Another button!"];

	const BUTTONS: [(&'static str, ButtonStyle); 2] = [
		("A button", ButtonStyle::Success),
		("Another button!", ButtonStyle::Danger),
	];

	fn parse(_: Interaction<'_>, value: &str) -> Self::Output {
		let components = Self::define_buttons().unwrap_or_else(|_| supernova::debug_unreachable!());

		components
			.iter()
			.find(|button| button.custom_id.as_deref() == Some(value))
			.unwrap_or_else(|| supernova::debug_unreachable!())
			.label
			.clone()
			.unwrap()
	}
}
