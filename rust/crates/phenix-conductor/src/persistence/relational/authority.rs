fn filesystem_token(authority: FilesystemAuthority) -> &'static str {
    match authority {
        FilesystemAuthority::ReadOnly => "read_only",
        FilesystemAuthority::Write => "write",
    }
}

fn network_token(authority: NetworkAuthority) -> &'static str {
    match authority {
        NetworkAuthority::None => "none",
        NetworkAuthority::Outbound => "outbound",
    }
}

fn repository_token(authority: RepositoryAuthority) -> &'static str {
    match authority {
        RepositoryAuthority::Read => "read",
        RepositoryAuthority::Write => "write",
    }
}
