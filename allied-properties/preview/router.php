<?php
/**
 * Router for PHP's built-in server so WordPress pretty permalinks work
 * without Apache/.htaccess.  Used by start.sh:  php -S 0.0.0.0:PORT router.php
 *
 * Static assets are served by this router directly (with correct MIME types)
 * so it does not depend on the server's document root.
 */
$root = getenv( 'ALLIED_WP_ROOT' );
if ( ! $root ) { $root = __DIR__ . '/.runtime/web'; }
$root = realpath( $root );

$path = parse_url( $_SERVER['REQUEST_URI'], PHP_URL_PATH );
$file = realpath( $root . $path );

if ( $file && strpos( $file, $root ) === 0 && is_file( $file ) ) {
	$ext = strtolower( pathinfo( $file, PATHINFO_EXTENSION ) );

	// Real PHP files (wp-admin, wp-login, etc.) -> execute.
	if ( 'php' === $ext ) {
		chdir( dirname( $file ) );
		require $file;
		return true;
	}

	// Static asset -> stream with a sensible content type.
	$mimes = array(
		'css'  => 'text/css',
		'js'   => 'application/javascript',
		'json' => 'application/json',
		'png'  => 'image/png',
		'jpg'  => 'image/jpeg',
		'jpeg' => 'image/jpeg',
		'gif'  => 'image/gif',
		'svg'  => 'image/svg+xml',
		'webp' => 'image/webp',
		'ico'  => 'image/x-icon',
		'woff' => 'font/woff',
		'woff2'=> 'font/woff2',
		'ttf'  => 'font/ttf',
		'eot'  => 'application/vnd.ms-fontobject',
		'pdf'  => 'application/pdf',
		'txt'  => 'text/plain',
		'map'  => 'application/json',
	);
	if ( isset( $mimes[ $ext ] ) ) {
		header( 'Content-Type: ' . $mimes[ $ext ] );
	}
	header( 'Content-Length: ' . filesize( $file ) );
	readfile( $file );
	return true;
}

// Everything else -> WordPress front controller.
chdir( $root );
require $root . '/index.php';
return true;
