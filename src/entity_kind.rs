#[derive(Debug, Default)]
pub enum EntityKind {
    /// Used for entities that don't have any specific data.
    ///
    /// e.g., mall entry counter
    #[default]
    Generic,
    /// Used for entities that have lifetime and quantity.
    ///
    /// e.g., stock room, department store, medicine storage
    Inventory,
    /// Used for entities that have lifetime and quantity. This is a special case of inventory
    /// for recipe suggestions
    ///
    /// e.g., food storage
    Refrigerator,
    /// Used for entities for tracking attendance. Unauthorized entities are not allowed to enter.
    ///
    /// e.g., classroom, meeting room, school gates, establishment entry
    Attendance,
}
