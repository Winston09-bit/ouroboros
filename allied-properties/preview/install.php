<?php
/**
 * Idempotent WordPress install for the preview environment.
 * Usage:  php install.php /path/to/webroot
 */
$root = $argv[1] ?? null;
if ( ! $root || ! file_exists( "$root/wp-load.php" ) ) {
	fwrite( STDERR, "webroot not found\n" );
	exit( 1 );
}

define( 'WP_INSTALLING', true );
require "$root/wp-load.php";
require "$root/wp-admin/includes/upgrade.php";

if ( is_blog_installed() ) {
	echo "WordPress already installed — skipping.\n";
	exit( 0 );
}

$title    = 'Allied Properties';
$user     = getenv( 'ALLIED_ADMIN_USER' ) ?: 'admin';
$password = getenv( 'ALLIED_ADMIN_PASS' ) ?: 'Allied!Preview123';
$email    = getenv( 'ALLIED_ADMIN_EMAIL' ) ?: 'admin@alliedproperties.example';

$result = wp_install( $title, $user, $email, false, '', $password );
if ( is_wp_error( $result ) ) {
	fwrite( STDERR, 'install failed: ' . $result->get_error_message() . "\n" );
	exit( 1 );
}
echo "WordPress installed (admin user: $user).\n";
