/// Team member descriptor.
#[derive(Debug, Clone)]
pub struct TeamMember {
    pub agent_id: String,
    pub name: String,
    pub model: String,
}

/// Team creation and member management (Phase 2).
pub struct Team {
    pub name: String,
    pub members: Vec<TeamMember>,
}

impl Team {
    pub fn new(name: String) -> Self {
        Self {
            name,
            members: Vec::new(),
        }
    }

    pub fn add_member(&mut self, member: TeamMember) {
        self.members.push(member);
    }

    pub fn get_member(&self, name: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.name == name)
    }
}
