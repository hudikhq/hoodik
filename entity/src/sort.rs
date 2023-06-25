use sea_orm::{EntityTrait, Order, Select};

pub trait Sortable {
    type Entity: EntityTrait;

    /// Setup sorting on the query
    fn sort(&self, query: Select<Self::Entity>, order: Order) -> Select<Self::Entity>;

    /// Sort in the ASC direction
    fn sort_asc(&self, query: Select<Self::Entity>) -> Select<Self::Entity> {
        self.sort(query, Order::Asc)
    }

    /// Sort in the DESC direction
    fn sort_desc(&self, query: Select<Self::Entity>) -> Select<Self::Entity> {
        self.sort(query, Order::Desc)
    }
}
