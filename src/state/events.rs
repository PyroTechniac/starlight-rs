use std::convert::Infallible;

#[cfg(not(debug_assertions))]
use starchart::backend::RonBackend;
#[cfg(debug_assertions)]
use starchart::backend::RonPrettyBackend as RonBackend;
use starchart::{action::CreateEntryAction, Action, ChartResult};
use tracing::{event, Level};
use twilight_gateway::Event;
use twilight_model::{
	application::interaction::Interaction,
	gateway::payload::incoming::{InteractionCreate, Ready},
	guild::Guild,
};

use super::Context;
use crate::{prelude::*, settings::GuildSettings};

// these should all be the same caller context, taking a `Context` as the first parameter, and whatever the event content is in the second.
// however, they should return as strict of an error type as possible, using `Infallible` whevever possible (for more optimizations).
pub(super) async fn handle(context: Context, event: Event) -> MietteResult<()> {
	match event {
		Event::Ready(e) => ready(context, *e).await.into_diagnostic(),
		Event::GuildCreate(e) => guild_create(context, (*e).0).await.into_diagnostic(),
		Event::InteractionCreate(e) => {
			interaction_create(context, *e).await;
			Ok(())
		}
		_ => Ok(()),
	}
}

#[allow(clippy::unused_async)]
async fn ready(_: Context, ready: Ready) -> Result<(), Infallible> {
	event!(Level::INFO, user_name = %ready.user.name);
	event!(Level::INFO, guilds = %ready.guilds.len());
	Ok(())
}

async fn guild_create(context: Context, guild: Guild) -> ChartResult<(), RonBackend> {
	let id = guild.id;
	let database = context.database();

	let mut action: CreateEntryAction<GuildSettings> = Action::new();

	action
		.set_entry(&GuildSettings::new(id))
		.set_table("guilds");

	database.run(action).await??;

	Ok(())
}

async fn interaction_create(context: Context, interaction: InteractionCreate) {
	match interaction.0 {
		Interaction::ApplicationCommand(cmd) | Interaction::ApplicationCommandAutocomplete(cmd) => {
			context.helpers().interactions().handle(*cmd).await;
		}
		Interaction::MessageComponent(_) => {}
		i => event!(Level::WARN, ?i, "unhandled interaction"),
	}
}
