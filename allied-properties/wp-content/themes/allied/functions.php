<?php
/**
 * Allied Properties theme bootstrap.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

define( 'ALLIED_DIR', get_template_directory() );

$allied_includes = array(
	'/inc/theme-setup.php',
	'/inc/enqueue.php',
	'/inc/cpt-community.php',
	'/inc/portal-roles.php',
	'/inc/template-functions.php',
	'/inc/performance.php',
);

foreach ( $allied_includes as $file ) {
	$path = ALLIED_DIR . $file;
	if ( file_exists( $path ) ) {
		require_once $path;
	}
}
