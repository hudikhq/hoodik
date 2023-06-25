pub(crate) mod files;
pub(crate) mod invitations;
pub(crate) mod sessions;
pub(crate) mod users;

use context::Context;
use entity::ConnectionTrait;

pub(crate) struct Repository<'ctx, T: ConnectionTrait> {
    context: &'ctx Context,
    connection: &'ctx T,
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(context: &'ctx Context, connection: &'ctx T) -> Self {
        Self {
            context,
            connection,
        }
    }

    pub(crate) fn connection(&self) -> &'ctx T {
        self.connection
    }

    pub(crate) fn context(&self) -> &'ctx Context {
        self.context
    }

    pub(crate) fn files<'repository>(&'ctx self) -> files::FilesRepository<'repository, T>
    where
        Self: 'repository,
    {
        files::FilesRepository::new(self)
    }

    pub(crate) fn invitations<'repository>(
        &'ctx self,
    ) -> invitations::InvitationsRepository<'repository, T>
    where
        Self: 'repository,
    {
        invitations::InvitationsRepository::new(self)
    }

    pub(crate) fn sessions<'repository>(&'ctx self) -> sessions::SessionsRepository<'repository, T>
    where
        Self: 'repository,
    {
        sessions::SessionsRepository::new(self)
    }

    pub(crate) fn users<'repository>(&'ctx self) -> users::UsersRepository<'repository, T>
    where
        Self: 'repository,
    {
        users::UsersRepository::new(self)
    }
}
