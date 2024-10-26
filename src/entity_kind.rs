#[derive(Debug, Default)]
pub enum EntityKind {
    /// Used for entities that don't have any specific data.
    ///
    /// e.g., mall entry counter
    #[default]
    Counter,
    /// Used for entities that have lifetime, location, and quantity.
    ///
    /// e.g., stock room, department store, medicine storage
    Inventory,
    /// Used for entities that have lifetime and quantity.
    ///
    /// This is experimental as it is labour-intensive to tag entities with this kind.
    ///
    /// e.g., food storage
    Refrigerator,
    /// Used for entities for tracking attendance. Unauthorized entities are not allowed to enter.
    ///
    /// e.g., classroom, meeting room, school gates, establishment entry
    Attendance,
}
