use crate::{
    entity::{
        channel::{
            AttachmentEntity, CategoryChannelEntity, GroupEntity, GuildChannelEntity,
            MessageEntity, MessageRepository, PrivateChannelEntity, TextChannelEntity,
            VoiceChannelEntity,
        },
        gateway::PresenceEntity,
        guild::{EmojiEntity, GuildEntity, GuildRepository, MemberEntity, RoleEntity},
        user::UserEntity,
        voice::VoiceStateEntity,
    },
    Backend, Repository,
};
use futures_util::{
    future::{self, FutureExt, TryFutureExt},
    stream::{FuturesUnordered, StreamExt, TryStreamExt},
};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use twilight_model::{
    channel::{Channel, GuildChannel},
    gateway::{
        event::Event,
        payload::{
            ChannelCreate, ChannelDelete, ChannelPinsUpdate, ChannelUpdate, GuildCreate,
            GuildDelete, GuildEmojisUpdate, GuildUpdate, MemberAdd, MemberChunk, MemberRemove,
            MemberUpdate, MessageCreate, MessageDelete,
        },
    },
};

fn noop<B: Backend>() -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send>> {
    future::ok(()).boxed()
}

pub trait CacheUpdate<B: Backend> {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>>;
}

pub struct ProcessFuture<'a, B: Backend> {
    inner: Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>>,
}

impl<B: Backend> Future for ProcessFuture<'_, B> {
    type Output = Result<(), B::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Cache<B: Backend> {
    backend: Arc<B>,
    pub attachments: B::AttachmentRepository,
    pub category_channels: B::CategoryChannelRepository,
    pub current_user: B::CurrentUserRepository,
    pub emojis: B::EmojiRepository,
    pub groups: B::GroupRepository,
    pub guilds: B::GuildRepository,
    pub members: B::MemberRepository,
    pub messages: B::MessageRepository,
    pub presences: B::PresenceRepository,
    pub private_channels: B::PrivateChannelRepository,
    pub roles: B::RoleRepository,
    pub stage_channels: B::StageChannelRepository,
    pub text_channels: B::TextChannelRepository,
    pub users: B::UserRepository,
    pub voice_channels: B::VoiceChannelRepository,
    pub voice_states: B::VoiceStateRepository,
}

impl<B: Backend + Default> Cache<B> {
    pub fn new() -> Self {
        Self::with_backend(B::default())
    }
}

impl<B: Backend> Cache<B> {
    pub fn with_backend(backend: impl Into<Arc<B>>) -> Self {
        let backend: Arc<B> = backend.into();
        let attachments = backend.attachments();
        let category_channels = backend.category_channels();
        let current_user = backend.current_user();
        // This is here to fix rust-analyzer in thinking all of these require arguments
        let emojis: B::EmojiRepository = backend.emojis();
        let groups = backend.groups();
        let guilds = backend.guilds();
        let members = backend.members();
        let messages = backend.messages();
        let presences = backend.presences();
        let private_channels = backend.private_channels();
        let roles = backend.roles();
        let stage_channels = backend.stage_channels();
        let text_channels = backend.text_channels();
        let users = backend.users();
        let voice_channels = backend.voice_channels();
        let voice_states = backend.voice_states();

        Self {
            attachments,
            backend,
            category_channels,
            current_user,
            emojis,
            groups,
            guilds,
            members,
            messages,
            presences,
            private_channels,
            roles,
            stage_channels,
            text_channels,
            users,
            voice_channels,
            voice_states,
        }
    }

    pub fn backend(&self) -> &Arc<B> {
        &self.backend
    }

    pub fn process<'a>(&'a self, event: &'a Event) -> ProcessFuture<'a, B> {
        ProcessFuture {
            inner: event.process(self),
        }
    }
}

impl<B: Backend> CacheUpdate<B> for Event {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        match self {
            Self::BanAdd(_) => noop::<B>(),
            Self::BanRemove(_) => noop::<B>(),
            _ => todo!(),
        }
    }
}

impl<B: Backend> CacheUpdate<B> for ChannelCreate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        match &self.0 {
            Channel::Group(group) => {
                let futures = FuturesUnordered::new();

                futures.push(
                    cache
                        .users
                        .upsert_bulk(group.recipients.iter().cloned().map(UserEntity::from)),
                );

                let entity = GroupEntity::from(group.clone());
                futures.push(cache.groups.upsert(entity));

                futures.try_collect().boxed()
            }
            Channel::Guild(GuildChannel::Category(c)) => {
                let entity = CategoryChannelEntity::from(c.clone());

                cache.category_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Text(c)) => {
                let entity = TextChannelEntity::from(c.clone());

                cache.text_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Stage(c)) => {
                let entity = VoiceChannelEntity::from(c.clone());

                cache.stage_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Voice(c)) => {
                let entity = VoiceChannelEntity::from(c.clone());

                cache.voice_channels.upsert(entity)
            }
            Channel::Private(c) => {
                let futures = FuturesUnordered::new();

                futures.push(
                    cache
                        .users
                        .upsert_bulk(c.recipients.iter().cloned().map(UserEntity::from)),
                );

                let entity = PrivateChannelEntity::from(c.clone());
                futures.push(cache.private_channels.upsert(entity));

                futures.try_collect().boxed()
            }
        }
    }
}

impl<B: Backend> CacheUpdate<B> for ChannelDelete {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        match &self.0 {
            Channel::Group(group) => cache.groups.remove(group.id),
            Channel::Guild(GuildChannel::Category(c)) => cache.category_channels.remove(c.id),
            Channel::Guild(GuildChannel::Text(c)) => cache.text_channels.remove(c.id),
            Channel::Guild(GuildChannel::Stage(c)) => cache.stage_channels.remove(c.id),
            Channel::Guild(GuildChannel::Voice(c)) => cache.voice_channels.remove(c.id),
            Channel::Private(c) => cache.private_channels.remove(c.id),
        }
    }
}

impl<B: Backend> CacheUpdate<B> for ChannelPinsUpdate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(group) = cache.groups.get(self.channel_id).await? {
                return cache
                    .groups
                    .upsert(GroupEntity {
                        last_pin_timestamp: self.last_pin_timestamp.clone(),
                        ..group
                    })
                    .await;
            }

            if let Some(text_channel) = cache.text_channels.get(self.channel_id).await? {
                return cache
                    .text_channels
                    .upsert(TextChannelEntity {
                        last_pin_timestamp: self.last_pin_timestamp.clone(),
                        ..text_channel
                    })
                    .await;
            }

            if let Some(private_channel) = cache.private_channels.get(self.channel_id).await? {
                return cache
                    .private_channels
                    .upsert(PrivateChannelEntity {
                        last_pin_timestamp: self.last_pin_timestamp.clone(),
                        ..private_channel
                    })
                    .await;
            }

            Ok(())
        })
    }
}

impl<B: Backend> CacheUpdate<B> for ChannelUpdate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        match &self.0 {
            Channel::Group(group) => {
                let futures = FuturesUnordered::new();

                futures.push(
                    cache
                        .users
                        .upsert_bulk(group.recipients.iter().cloned().map(UserEntity::from)),
                );

                let entity = GroupEntity::from(group.clone());

                futures.push(cache.groups.upsert(entity));

                futures.try_collect().boxed()
            }
            Channel::Guild(GuildChannel::Category(c)) => {
                let entity = CategoryChannelEntity::from(c.clone());

                cache.category_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Text(c)) => {
                let entity = TextChannelEntity::from(c.clone());

                cache.text_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Stage(c)) => {
                let entity = VoiceChannelEntity::from(c.clone());

                cache.stage_channels.upsert(entity)
            }
            Channel::Guild(GuildChannel::Voice(c)) => {
                let entity = VoiceChannelEntity::from(c.clone());

                cache.voice_channels.upsert(entity)
            }
            Channel::Private(c) => {
                let futures = FuturesUnordered::new();

                futures.push(
                    cache
                        .users
                        .upsert_bulk(c.recipients.iter().cloned().map(UserEntity::from)),
                );

                let entity = PrivateChannelEntity::from(c.clone());
                futures.push(cache.private_channels.upsert(entity));

                futures.try_collect().boxed()
            }
        }
    }
}

impl<B: Backend> CacheUpdate<B> for GuildCreate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        let futures = FuturesUnordered::new();

        for channel in self.channels.iter() {
            match channel {
                GuildChannel::Category(c) => {
                    let entity = CategoryChannelEntity::from(c.clone());
                    futures.push(cache.category_channels.upsert(entity));
                }
                GuildChannel::Text(c) => {
                    let entity = TextChannelEntity::from(c.clone());
                    futures.push(cache.text_channels.upsert(entity))
                }
                GuildChannel::Stage(c) => {
                    let entity = VoiceChannelEntity::from(c.clone());
                    futures.push(cache.stage_channels.upsert(entity));
                }
                GuildChannel::Voice(c) => {
                    let entity = VoiceChannelEntity::from(c.clone());
                    futures.push(cache.voice_channels.upsert(entity));
                }
            }
        }

        futures.push(
            cache.emojis.upsert_bulk(
                self.emojis
                    .iter()
                    .cloned()
                    .map(|e| EmojiEntity::from((self.id, e))),
            ),
        );

        futures.push(
            cache
                .members
                .upsert_bulk(self.members.iter().cloned().map(MemberEntity::from)),
        );

        futures.push(
            cache.users.upsert_bulk(
                self.members
                    .iter()
                    .cloned()
                    .map(|m| UserEntity::from(m.user)),
            ),
        );

        futures.push(
            cache
                .presences
                .upsert_bulk(self.presences.iter().cloned().map(PresenceEntity::from)),
        );

        futures.push(
            cache.roles.upsert_bulk(
                self.roles
                    .iter()
                    .cloned()
                    .map(|r| RoleEntity::from((r, self.id))),
            ),
        );

        futures.push(
            cache.voice_states.upsert_bulk(
                self.voice_states
                    .iter()
                    .cloned()
                    .map(|v| VoiceStateEntity::from((v, self.id))),
            ),
        );

        let entity = GuildEntity::from(self.0.clone());
        futures.push(cache.guilds.upsert(entity));

        futures.try_collect().boxed()
    }
}

impl<B: Backend> CacheUpdate<B> for GuildDelete {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        if self.unavailable {
            return cache
                .guilds
                .get(self.id)
                .and_then(move |guild| {
                    guild.map_or_else(
                        || future::ok(()).boxed(),
                        |guild| {
                            let entity = GuildEntity {
                                unavailable: self.unavailable,
                                ..guild
                            };

                            cache.guilds.upsert(entity)
                        },
                    )
                })
                .boxed();
        }

        Box::pin(async move {
            let futures = FuturesUnordered::new();

            let mut channels = cache.guilds.channels(self.id).await?;

            while let Some(Ok(c)) = channels.next().await {
                match c {
                    GuildChannelEntity::Category(c) => {
                        futures.push(cache.category_channels.remove(c.id));
                    }
                    GuildChannelEntity::Text(c) => {
                        futures.push(cache.text_channels.remove(c.id));
                    }
                    GuildChannelEntity::Voice(c) => futures.push(cache.voice_channels.remove(c.id)),
                }
            }

            let mut emojis = cache.guilds.emoji_ids(self.id).await?;

            while let Some(Ok(id)) = emojis.next().await {
                futures.push(cache.emojis.remove(id));
            }

            let mut members = cache.guilds.member_ids(self.id).await?;

            while let Some(Ok(id)) = members.next().await {
                futures.push(cache.members.remove((self.id, id)));
            }

            let mut presences = cache.guilds.presence_ids(self.id).await?;

            while let Some(Ok(id)) = presences.next().await {
                futures.push(cache.presences.remove((self.id, id)));
            }

            let mut roles = cache.guilds.role_ids(self.id).await?;

            while let Some(Ok(id)) = roles.next().await {
                futures.push(cache.roles.remove(id));
            }

            let mut voice_states = cache.guilds.voice_state_ids(self.id).await?;

            while let Some(Ok(id)) = voice_states.next().await {
                futures.push(cache.voice_states.remove((self.id, id)));
            }

            futures.try_collect::<()>().await?;
            cache.guilds.remove(self.id).await
        })
    }
}

impl<B: Backend> CacheUpdate<B> for GuildEmojisUpdate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        cache.emojis.upsert_bulk(
            self.emojis
                .iter()
                .cloned()
                .map(|e| EmojiEntity::from((self.guild_id, e))),
        )
    }
}

impl<B: Backend> CacheUpdate<B> for GuildUpdate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        cache
            .guilds
            .get(self.id)
            .and_then(move |guild| {
                guild.map_or_else(
                    || future::ok(()).boxed(),
                    |guild| cache.guilds.upsert(guild.update(self.0.clone())),
                )
            })
            .boxed()
    }
}

impl<B: Backend> CacheUpdate<B> for MemberAdd {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        let futures = FuturesUnordered::new();

        let user_entity = UserEntity::from(self.user.clone());
        futures.push(cache.users.upsert(user_entity));

        let member_entity = MemberEntity::from(self.0.clone());
        futures.push(cache.members.upsert(member_entity));

        futures.try_collect().boxed()
    }
}

impl<B: Backend> CacheUpdate<B> for MemberRemove {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        cache.members.remove((self.guild_id, self.user.id))
    }
}

impl<B: Backend> CacheUpdate<B> for MemberUpdate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        cache
            .members
            .get((self.guild_id, self.user.id))
            .and_then(move |member| {
                member.map_or_else(
                    || future::ok(()).boxed(),
                    |member| {
                        let futures = FuturesUnordered::new();

                        let user_entity = UserEntity::from(self.user.clone());
                        futures.push(cache.users.upsert(user_entity));

                        futures.push(cache.members.upsert(member.update(self.clone())));

                        futures.try_collect().boxed()
                    },
                )
            })
            .boxed()
    }
}

impl<B: Backend> CacheUpdate<B> for MemberChunk {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        let futures = FuturesUnordered::new();

        futures.push(
            cache
                .members
                .upsert_bulk(self.members.iter().cloned().map(MemberEntity::from)),
        );

        futures.push(
            cache.users.upsert_bulk(
                self.members
                    .iter()
                    .cloned()
                    .map(|m| UserEntity::from(m.user)),
            ),
        );

        futures.push(
            cache
                .presences
                .upsert_bulk(self.presences.iter().cloned().map(PresenceEntity::from)),
        );

        futures.try_collect().boxed()
    }
}

impl<B: Backend> CacheUpdate<B> for MessageCreate {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        Box::pin(async move {
            let futures = FuturesUnordered::new();

            if let Some(group) = cache.groups.get(self.channel_id).await? {
                futures.push(cache.groups.upsert(GroupEntity {
                    last_message_id: Some(self.id),
                    ..group
                }));
            }

            if let Some(text_channel) = cache.text_channels.get(self.channel_id).await? {
                futures.push(cache.text_channels.upsert(TextChannelEntity {
                    last_message_id: Some(self.id),
                    ..text_channel
                }));
            }

            if let Some(private_channel) = cache.private_channels.get(self.channel_id).await? {
                futures.push(cache.private_channels.upsert(PrivateChannelEntity {
                    last_message_id: Some(self.id),
                    ..private_channel
                }));
            }

            for attachment in self.0.attachments.iter().cloned() {
                let entity = AttachmentEntity::from((self.id, attachment));

                futures.push(cache.attachments.upsert(entity));
            }

            let entity = MessageEntity::from(self.0.clone());
            futures.push(cache.messages.upsert(entity));

            futures.try_collect().await
        })
    }
}

impl<B: Backend> CacheUpdate<B> for MessageDelete {
    fn process<'a>(
        &'a self,
        cache: &'a Cache<B>,
    ) -> Pin<Box<dyn Future<Output = Result<(), B::Error>> + Send + 'a>> {
        Box::pin(async move {
            let futures = FuturesUnordered::new();

            let mut attachments = cache.messages.attachments(self.id).await?;

            while let Some(Ok(attachment)) = attachments.next().await {
                futures.push(cache.attachments.remove(attachment.id));
            }

            futures.try_collect::<()>().await?;
            cache.messages.remove(self.id).await
        })
    }
}
