use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Classifiers::Table)
                    .col(
                        ColumnDef::new(Classifiers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Classifiers::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Classifiers::MatchAll)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Classifiers::ConditionsJson)
                            .json()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Classifiers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Classifiers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Libraries::Table)
                    .col(
                        ColumnDef::new(Libraries::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Libraries::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Libraries::Kind)
                            .string()
                            .not_null()
                            .check(Expr::col(Libraries::Kind).is_in(["movie", "collection"])),
                    )
                    .col(
                        ColumnDef::new(Libraries::RootPath)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Libraries::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Libraries::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Entities::Table)
                    .col(ColumnDef::new(Entities::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Entities::LibraryId).uuid())
                    .col(
                        ColumnDef::new(Entities::Kind)
                            .string()
                            .not_null()
                            .check(Expr::col(Entities::Kind).is_in(["movie", "collection"])),
                    )
                    .col(ColumnDef::new(Entities::Title).string().not_null())
                    .col(ColumnDef::new(Entities::OriginalTitle).string())
                    .col(ColumnDef::new(Entities::OriginalLanguage).string())
                    .col(
                        ColumnDef::new(Entities::Monitored)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Entities::MetadataJson)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Entities::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Entities::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .index(
                        Index::create()
                            .name("uq_entities_id_kind")
                            .col(Entities::Id)
                            .col(Entities::Kind)
                            .unique(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Entities::Table, Entities::LibraryId)
                            .to(Libraries::Table, Libraries::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_entities_kind_title")
                    .table(Entities::Table)
                    .col(Entities::Kind)
                    .col(Entities::Title)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_entities_library_title")
                    .table(Entities::Table)
                    .col(Entities::LibraryId)
                    .col(Entities::Title)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EntityExternalIds::Table)
                    .col(
                        ColumnDef::new(EntityExternalIds::EntityId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EntityExternalIds::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EntityExternalIds::EntityKind)
                            .string()
                            .not_null()
                            .check(
                                Expr::col(EntityExternalIds::EntityKind)
                                    .is_in(["movie", "collection"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(EntityExternalIds::ExternalId)
                            .string()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(EntityExternalIds::EntityId)
                            .col(EntityExternalIds::Provider)
                            .col(EntityExternalIds::ExternalId),
                    )
                    .index(
                        Index::create()
                            .name("uq_entity_ext_provider_kind_id")
                            .col(EntityExternalIds::Provider)
                            .col(EntityExternalIds::EntityKind)
                            .col(EntityExternalIds::ExternalId)
                            .unique(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(
                                EntityExternalIds::Table,
                                (
                                    EntityExternalIds::EntityId,
                                    EntityExternalIds::EntityKind,
                                ),
                            )
                            .to(Entities::Table, (Entities::Id, Entities::Kind))
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_entity_ext_ids_lookup")
                    .table(EntityExternalIds::Table)
                    .col(EntityExternalIds::Provider)
                    .col(EntityExternalIds::ExternalId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Movies::Table)
                    .col(
                        ColumnDef::new(Movies::EntityId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Movies::Year).integer().check(
                            Expr::col(Movies::Year)
                                .is_null()
                                .or(Expr::col(Movies::Year).between(0, 65535)),
                        ),
                    )
                    .col(ColumnDef::new(Movies::InCinemasDate).date())
                    .col(ColumnDef::new(Movies::DigitalReleaseDate).date())
                    .col(ColumnDef::new(Movies::PhysicalReleaseDate).date())
                    .col(
                        ColumnDef::new(Movies::RuntimeMinutes).integer().check(
                            Expr::col(Movies::RuntimeMinutes)
                                .is_null()
                                .or(Expr::col(Movies::RuntimeMinutes).gte(0)),
                        ),
                    )
                    .col(ColumnDef::new(Movies::Certification).string())
                    .col(ColumnDef::new(Movies::Studio).string())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Movies::Table, Movies::EntityId)
                            .to(Entities::Table, Entities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Collections::Table)
                    .col(
                        ColumnDef::new(Collections::EntityId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Collections::Table, Collections::EntityId)
                            .to(Entities::Table, Entities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CollectionMovies::Table)
                    .col(
                        ColumnDef::new(CollectionMovies::CollectionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CollectionMovies::MovieId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(CollectionMovies::CollectionId)
                            .col(CollectionMovies::MovieId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CollectionMovies::Table, CollectionMovies::CollectionId)
                            .to(Collections::Table, Collections::EntityId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CollectionMovies::Table, CollectionMovies::MovieId)
                            .to(Movies::Table, Movies::EntityId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MediaAssets::Table)
                    .col(
                        ColumnDef::new(MediaAssets::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MediaAssets::Path)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(MediaAssets::SizeBytes)
                            .big_integer()
                            .not_null()
                            .check(Expr::col(MediaAssets::SizeBytes).gte(0)),
                    )
                    .col(ColumnDef::new(MediaAssets::QualityScore).integer())
                    .col(
                        ColumnDef::new(MediaAssets::MatchedClassifierIdsJson)
                            .json()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(MediaAssets::MediaInfoJson)
                            .json()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(MediaAssets::ImportedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EntityAssets::Table)
                    .col(ColumnDef::new(EntityAssets::EntityId).uuid().not_null())
                    .col(ColumnDef::new(EntityAssets::AssetId).uuid().not_null())
                    .col(
                        ColumnDef::new(EntityAssets::Position)
                            .integer()
                            .not_null()
                            .default(0)
                            .check(Expr::col(EntityAssets::Position).gte(0)),
                    )
                    .primary_key(
                        Index::create()
                            .col(EntityAssets::EntityId)
                            .col(EntityAssets::AssetId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(EntityAssets::Table, EntityAssets::EntityId)
                            .to(Entities::Table, Entities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(EntityAssets::Table, EntityAssets::AssetId)
                            .to(MediaAssets::Table, MediaAssets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_entity_assets_asset")
                    .table(EntityAssets::Table)
                    .col(EntityAssets::AssetId)
                    .col(EntityAssets::Position)
                    .col(EntityAssets::EntityId)
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX idx_entities_monitored
                    ON entities(monitored, kind)
                    WHERE monitored = 1;

                CREATE TRIGGER trg_cleanup_orphaned_media_assets
                AFTER DELETE ON entity_assets
                WHEN NOT EXISTS (SELECT 1 FROM entity_assets WHERE asset_id = OLD.asset_id)
                BEGIN
                    DELETE FROM media_assets WHERE id = OLD.asset_id;
                END;
                "#,
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Users::Username).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(ColumnDef::new(Users::IsAdmin).boolean().not_null().default(false))
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserLibraries::Table)
                    .col(ColumnDef::new(UserLibraries::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserLibraries::LibraryId).uuid().not_null())
                    .primary_key(
                        Index::create()
                            .col(UserLibraries::UserId)
                            .col(UserLibraries::LibraryId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserLibraries::Table, UserLibraries::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserLibraries::Table, UserLibraries::LibraryId)
                            .to(Libraries::Table, Libraries::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS trg_cleanup_orphaned_media_assets;")
            .await?;

        for table in [
            UserLibraries::Table.into_table_ref(),
            Users::Table.into_table_ref(),
            EntityAssets::Table.into_table_ref(),
            MediaAssets::Table.into_table_ref(),
            CollectionMovies::Table.into_table_ref(),
            Collections::Table.into_table_ref(),
            Movies::Table.into_table_ref(),
            EntityExternalIds::Table.into_table_ref(),
            Entities::Table.into_table_ref(),
            Libraries::Table.into_table_ref(),
            Classifiers::Table.into_table_ref(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Classifiers {
    Table,
    Id,
    Name,
    MatchAll,
    ConditionsJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Libraries {
    Table,
    Id,
    Name,
    Kind,
    RootPath,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Entities {
    Table,
    Id,
    LibraryId,
    Kind,
    Title,
    OriginalTitle,
    OriginalLanguage,
    Monitored,
    MetadataJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum EntityExternalIds {
    Table,
    EntityId,
    Provider,
    EntityKind,
    ExternalId,
}

#[derive(DeriveIden)]
enum Movies {
    Table,
    EntityId,
    Year,
    InCinemasDate,
    DigitalReleaseDate,
    PhysicalReleaseDate,
    RuntimeMinutes,
    Certification,
    Studio,
}

#[derive(DeriveIden)]
enum Collections {
    Table,
    EntityId,
}

#[derive(DeriveIden)]
enum CollectionMovies {
    Table,
    CollectionId,
    MovieId,
}

#[derive(DeriveIden)]
enum MediaAssets {
    Table,
    Id,
    Path,
    SizeBytes,
    QualityScore,
    MatchedClassifierIdsJson,
    MediaInfoJson,
    ImportedAt,
}

#[derive(DeriveIden)]
enum EntityAssets {
    Table,
    EntityId,
    AssetId,
    Position,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
    IsAdmin,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserLibraries {
    Table,
    UserId,
    LibraryId,
}
