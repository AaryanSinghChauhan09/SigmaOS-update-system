# SigmaOS Update System

SigmaOS rolling release update system with automatic updates and rollback capabilities.

## Features

- **Rolling Releases** - Continuous rolling release model
- **Multiple Channels** - Stable, beta, and nightly channels
- **Auto Updates** - Automatic update checking and installation
- **Rollback** - Rollback to previous versions
- **Notifications** - Desktop notifications for updates
- **Verification** - Package signature verification
- **Dry Run** - Preview updates before installation
- **Configuration** - Flexible update configuration

## Installation

```bash
cargo install sigmaos-update-system
```

## Usage

### Check for Updates

```bash
sigmaos-update check --channel stable
sigmaos-update check --channel beta --force
```

### Update System

```bash
sigmaos-update update --channel stable
sigmaos-update update --channel nightly --dry-run
```

### Rollback

```bash
sigmaos-update rollback --list
sigmaos-update rollback --version 1.2.3
sigmaos-update rollback
```

### Show Status

```bash
sigmaos-update status
sigmaos-update status --detailed
```

### Configure Updates

```bash
sigmaos-update config --channel stable --auto-update true --interval 86400
```

## Update Channels

### Stable

Production-ready releases with bug fixes and security updates.

### Beta

Pre-release versions with new features for testing.

### Nightly

Daily builds with latest changes for developers.

### Custom

Custom update channels for specific use cases.

## Configuration

### Update Configuration

```toml
[channel]
default = "stable"

[auto_update]
enabled = false
check_interval = 86400
notify_updates = true

[channels]
stable = "https://updates.sigmaos.org/stable"
beta = "https://updates.sigmaos.org/beta"
nightly = "https://updates.sigmaos.org/nightly"
```

### Environment Variables

```bash
export SIGMAOS_UPDATE_CHANNEL=stable
export SIGMAOS_AUTO_UPDATE=true
export SIGMAOS_UPDATE_INTERVAL=86400
```

## Update Process

### Check Phase

1. Check current version
2. Fetch available updates
3. Compare versions
4. Notify user if updates available

### Download Phase

1. Download update package
2. Verify package signature
3. Check package integrity
4. Store package for installation

### Install Phase

1. Create system backup
2. Install update package
3. Update version information
4. Restart services if needed
5. Notify user of completion

### Rollback Phase

1. Create backup of current version
2. Download previous version package
3. Install previous version
4. Restore configuration
5. Notify user of rollback

## Security

### Package Verification

- GPG signature verification
- SHA-256 hash verification
- Package integrity checks
- Secure download channels

### Update Security

- Signed update packages
- Secure update servers
- Encrypted update channels
- Audit logging

## Notifications

### Update Notifications

- Update available
- Update downloading
- Update installing
- Update completed
- Update failed
- Rollback completed

### Notification Types

- Desktop notifications
- System tray notifications
- Email notifications (optional)
- SMS notifications (optional)

## Error Handling

### Update Errors

- Network errors
- Download errors
- Verification errors
- Installation errors
- Rollback errors

### Recovery

- Automatic rollback on failure
- Manual rollback option
- System restore point
- Configuration backup

## License

GPL-3.0-or-later
