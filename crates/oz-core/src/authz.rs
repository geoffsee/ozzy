use crate::models::{ApiKeyPermission, MemberRole};

pub fn can_read_member(role: MemberRole) -> bool {
    matches!(role, MemberRole::Read | MemberRole::Write | MemberRole::Admin)
}

pub fn can_write_member(role: MemberRole) -> bool {
    matches!(role, MemberRole::Write | MemberRole::Admin)
}

pub fn can_admin_member(role: MemberRole) -> bool {
    matches!(role, MemberRole::Admin)
}

pub fn can_read_api_key(permission: ApiKeyPermission) -> bool {
    matches!(permission, ApiKeyPermission::Read | ApiKeyPermission::Write)
}

pub fn can_write_api_key(permission: ApiKeyPermission) -> bool {
    matches!(permission, ApiKeyPermission::Write)
}

pub fn is_owner(profile_id: &str, owner_profile_id: &str) -> bool {
    profile_id == owner_profile_id
}

pub fn effective_admin(is_owner: bool, role: Option<MemberRole>) -> bool {
    is_owner || role.is_some_and(can_admin_member)
}

pub fn effective_read(is_owner: bool, role: Option<MemberRole>) -> bool {
    is_owner || role.is_some_and(can_read_member)
}

pub fn effective_write(is_owner: bool, role: Option<MemberRole>) -> bool {
    is_owner || role.is_some_and(can_write_member)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_has_all_permissions() {
        assert!(effective_admin(true, None));
        assert!(effective_read(true, None));
        assert!(effective_write(true, None));
    }

    #[test]
    fn read_member_cannot_write() {
        assert!(effective_read(false, Some(MemberRole::Read)));
        assert!(!effective_write(false, Some(MemberRole::Read)));
        assert!(!effective_admin(false, Some(MemberRole::Read)));
    }

    #[test]
    fn write_member_can_mutate_secrets() {
        assert!(effective_write(false, Some(MemberRole::Write)));
        assert!(!effective_admin(false, Some(MemberRole::Write)));
    }

    #[test]
    fn api_key_permissions() {
        assert!(can_read_api_key(ApiKeyPermission::Read));
        assert!(!can_write_api_key(ApiKeyPermission::Read));
        assert!(can_write_api_key(ApiKeyPermission::Write));
    }
}
