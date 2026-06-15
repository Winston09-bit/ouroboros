<?php
/**
 * Enqueue styles & scripts.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

function allied_enqueue_assets() {
	$theme   = wp_get_theme();
	$version = $theme->get( 'Version' );
	$uri     = get_template_directory_uri();

	// Modular CSS, loaded in cascade order. tokens.css first.
	wp_enqueue_style( 'allied-tokens',     "$uri/assets/css/tokens.css",     array(), $version );
	wp_enqueue_style( 'allied-base',       "$uri/assets/css/base.css",       array( 'allied-tokens' ), $version );
	wp_enqueue_style( 'allied-layout',     "$uri/assets/css/layout.css",     array( 'allied-base' ), $version );
	wp_enqueue_style( 'allied-components', "$uri/assets/css/components.css",  array( 'allied-layout' ), $version );

	// Main stylesheet (theme header) loaded last so it can override if needed.
	wp_enqueue_style( 'allied-style', get_stylesheet_uri(), array( 'allied-components' ), $version );

	wp_enqueue_script( 'allied-nav', "$uri/assets/js/navigation.js", array(), $version, true );

	if ( is_singular() && comments_open() && get_option( 'thread_comments' ) ) {
		wp_enqueue_script( 'comment-reply' );
	}
}
add_action( 'wp_enqueue_scripts', 'allied_enqueue_assets' );

/**
 * Self-hosted webfonts (preconnect not required since fonts are local).
 * Place final font files in assets/fonts/ and declare @font-face in tokens.css.
 * This keeps the build free of external API/CDN dependencies per the brief.
 */
