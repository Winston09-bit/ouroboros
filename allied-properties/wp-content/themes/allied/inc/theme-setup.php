<?php
/**
 * Theme setup: supports, menus, image sizes.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

function allied_setup() {
	load_theme_textdomain( 'allied', get_template_directory() . '/languages' );

	add_theme_support( 'title-tag' );
	add_theme_support( 'post-thumbnails' );
	add_theme_support( 'automatic-feed-links' );
	add_theme_support( 'html5', array( 'search-form', 'comment-form', 'comment-list', 'gallery', 'caption', 'style', 'script' ) );
	add_theme_support( 'responsive-embeds' );
	add_theme_support( 'editor-styles' );
	add_theme_support(
		'custom-logo',
		array(
			'height'      => 60,
			'width'       => 220,
			'flex-width'  => true,
			'flex-height' => true,
		)
	);

	// Photography-forward: register purpose-built crops.
	add_image_size( 'allied-hero', 2000, 1100, true );
	add_image_size( 'allied-card', 800, 600, true );
	add_image_size( 'allied-portrait', 600, 800, true );

	register_nav_menus(
		array(
			'primary' => __( 'Primary Navigation', 'allied' ),
			'footer'  => __( 'Footer Navigation', 'allied' ),
			'legal'   => __( 'Legal / Utility', 'allied' ),
		)
	);
}
add_action( 'after_setup_theme', 'allied_setup' );

/**
 * Content width.
 */
function allied_content_width() {
	$GLOBALS['content_width'] = 1240;
}
add_action( 'after_setup_theme', 'allied_content_width', 0 );

/**
 * Starter content — gives a fresh site the full Phase 1 page set, correct
 * templates, and a primary menu, so a non-technical admin starts from a
 * complete, editable structure rather than a blank install.
 *
 * (Applies on a fresh site via Appearance > Customize. See HANDOFF.md for a
 * manual checklist if starter content has already been dismissed.)
 */
function allied_starter_content() {
	$starter = array(
		'posts' => array(
			'home' => array(
				'post_type'  => 'page',
				'post_title' => __( 'Home', 'allied' ),
			),
			'the-firm' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'The Firm', 'allied' ),
				'post_name'    => 'the-firm',
				'post_excerpt' => __( 'A disciplined residential land developer operating across Northeastern North Carolina and Hampton Roads, Virginia.', 'allied' ),
				'post_content' => '<!-- wp:paragraph --><p>' . __( 'Allied Properties acquires, entitles, and develops residential land — delivering finished communities to national and regional homebuilders. Edit this introduction in the page editor.', 'allied' ) . '</p><!-- /wp:paragraph -->',
				'template'     => 'page-templates/template-firm.php',
			),
			'capabilities' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'Capabilities', 'allied' ),
				'post_name'    => 'capabilities',
				'post_excerpt' => __( 'From acquisition and entitlement to horizontal development and finished lot delivery.', 'allied' ),
				'template'     => 'page-templates/template-capabilities.php',
			),
			'partners' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'Partners', 'allied' ),
				'post_name'    => 'partners',
				'post_excerpt' => __( 'Builders, investors, and partners who help us bring quality communities to market.', 'allied' ),
				'template'     => 'page-templates/template-partners.php',
			),
			'landowners' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'Sell Us Your Land', 'allied' ),
				'post_name'    => 'sell-us-your-land',
				'post_excerpt' => __( 'Have residential land in our footprint? We would like to hear from you.', 'allied' ),
				'template'     => 'page-templates/template-landowner.php',
			),
			'contact' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'Contact', 'allied' ),
				'post_name'    => 'contact',
				'template'     => 'page-templates/template-contact.php',
			),
			'portals' => array(
				'post_type'    => 'page',
				'post_title'   => __( 'Portals', 'allied' ),
				'post_name'    => 'portals',
				'template'     => 'page-templates/template-portals.php',
			),
		),
		'options' => array(
			'show_on_front'  => 'page',
			'page_on_front'  => '{{home}}',
		),
		'nav_menus' => array(
			'primary' => array(
				'name'  => __( 'Primary Navigation', 'allied' ),
				'items' => array(
					'page_the-firm',
					'page_capabilities',
					'page_partners',
					'page_landowners',
					'page_contact',
				),
			),
		),
	);

	add_theme_support( 'starter-content', $starter );
}
add_action( 'after_setup_theme', 'allied_starter_content' );
