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

pub fn allows_api_key_permission(
    is_owner: bool,
    role: Option<MemberRole>,
    permission: ApiKeyPermission,
) -> bool {
    match permission {
        ApiKeyPermission::Read => effective_read(is_owner, role),
        ApiKeyPermission::Write => effective_write(is_owner, role),
    }
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

    #[test]
    fn api_key_scope_requires_membership_role() {
        assert!(allows_api_key_permission(false, Some(MemberRole::Read), ApiKeyPermission::Read));
        assert!(!allows_api_key_permission(
            false,
            Some(MemberRole::Read),
            ApiKeyPermission::Write
        ));
        assert!(allows_api_key_permission(
            false,
            Some(MemberRole::Write),
            ApiKeyPermission::Write
        ));
        assert!(!allows_api_key_permission(false, None, ApiKeyPermission::Read));
        assert!(allows_api_key_permission(true, None, ApiKeyPermission::Write));
    }
}
