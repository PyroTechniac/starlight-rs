pub mod current_user;

pub use self::current_user::{CurrentUserEntity, CurrentUserRepository};

use crate::{
    entity::Entity,
    repository::{ListEntitiesFuture, Repository},
    utils, Backend,
};
use serde::{Deserialize, Serialize};
use twilight_model::{
    id::{GuildId, UserId},
    user::{PremiumType, User, UserFlags},
};

use super::guild::GuildEntity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct UserEntity {
    pub avatar: Option<String>,
    pub bot: bool,
    pub discriminator: String,
    pub email: Option<String>,
    pub flags: Option<UserFlags>,
    pub id: UserId,
    pub locale: Option<String>,
    pub mfa_enabled: Option<bool>,
    pub name: String,
    pub premium_type: Option<PremiumType>,
    pub public_flags: Option<UserFlags>,
    pub system: Option<bool>,
    pub verified: Option<bool>,
}

impl From<User> for UserEntity {
    fn from(user: User) -> Self {
        Self {
            avatar: user.avatar,
            bot: user.bot,
            discriminator: user.discriminator,
            email: user.email,
            flags: user.flags,
            id: user.id,
            locale: user.locale,
            mfa_enabled: user.mfa_enabled,
            name: user.name,
            premium_type: user.premium_type,
            public_flags: user.public_flags,
            system: user.system,
            verified: user.verified,
        }
    }
}

impl Entity for UserEntity {
    type Id = UserId;

    fn id(&self) -> Self::Id {
        self.id
    }
}

pub trait UserRepository<B: Backend>: Repository<UserEntity, B> {
    fn guild_ids(&self, user_id: UserId) -> ListEntitiesFuture<'_, GuildId, B::Error>;

    fn guilds(&self, user_id: UserId) -> ListEntitiesFuture<'_, GuildEntity, B::Error> {
        utils::stream_ids(self.guild_ids(user_id), self.backend().guilds())
    }
}
