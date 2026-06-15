<?php
/**
 * Front-end performance & cleanup.
 *
 * Safe, reversible optimizations only — no visual or feature change:
 *  - Remove the wp-emoji detection script (an extra inline script + an external
 *    https://s.w.org request on every page load; browsers fall back to native
 *    emoji rendering).
 *  - Remove the wp-embed script (oEmbed of this site elsewhere — unused here).
 *  - Trim unused <head> clutter (RSD, wlwmanifest, generator).
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

/**
 * Disable the emoji detection script and styles.
 */
function allied_disable_emojis() {
	remove_action( 'wp_head', 'print_emoji_detection_script', 7 );
	remove_action( 'admin_print_scripts', 'print_emoji_detection_script' );
	remove_action( 'wp_print_styles', 'print_emoji_styles' );
	remove_action( 'admin_print_styles', 'print_emoji_styles' );
	remove_filter( 'the_content_feed', 'wp_staticize_emoji' );
	remove_filter( 'comment_text_rss', 'wp_staticize_emoji' );
	remove_filter( 'wp_mail', 'wp_staticize_emoji_for_email' );
	add_filter( 'tiny_mce_plugins', function ( $plugins ) {
		return is_array( $plugins ) ? array_diff( $plugins, array( 'wpemoji' ) ) : array();
	} );
	// Drop the s.w.org DNS prefetch the emoji loader adds.
	add_filter( 'wp_resource_hints', function ( $hints, $relation ) {
		if ( 'dns-prefetch' === $relation ) {
			$emoji = apply_filters( 'emoji_svg_url', 'https://s.w.org/images/core/emoji/' );
			$hints = array_diff( $hints, array( $emoji ) );
		}
		return $hints;
	}, 10, 2 );
}
add_action( 'init', 'allied_disable_emojis' );

/**
 * Remove unused front-end <head> output (no SEO/feature impact for this site).
 */
add_action( 'after_setup_theme', function () {
	remove_action( 'wp_head', 'rsd_link' );
	remove_action( 'wp_head', 'wlwmanifest_link' );
	remove_action( 'wp_head', 'wp_generator' );
	remove_action( 'wp_head', 'wp_shortlink_wp_head', 10 );
} );

/**
 * Dequeue the wp-embed script on the front end (oEmbed embedding — unused).
 */
add_action( 'wp_footer', function () {
	wp_dequeue_script( 'wp-embed' );
} );
