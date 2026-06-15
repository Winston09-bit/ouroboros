<?php
/**
 * Idempotent content seed for the preview environment: activates the theme,
 * registers the CPT/taxonomies/roles, creates all pages with the correct
 * templates, two sample communities, a primary menu, the portal user, sets the
 * homepage + pretty permalinks, and flushes rewrite rules.
 *
 * Usage:  php seed.php /path/to/webroot
 */
$root = $argv[1] ?? null;
if ( ! $root || ! file_exists( "$root/wp-load.php" ) ) {
	fwrite( STDERR, "webroot not found\n" );
	exit( 1 );
}
require "$root/wp-load.php";

// Activate theme.
switch_theme( 'allied' );

// Make sure theme registration runs in THIS process even on a fresh install
// (init has already fired, so call the registrars directly; require_once is a
// no-op if functions.php already loaded them).
$inc = "$root/wp-content/themes/allied/inc";
require_once "$inc/cpt-community.php";
require_once "$inc/portal-roles.php";
foreach ( array( 'allied_register_community_cpt', 'allied_register_community_taxes', 'allied_register_portal_roles' ) as $fn ) {
	if ( function_exists( $fn ) ) { $fn(); }
}

echo 'Theme: ' . wp_get_theme()->get( 'Name' ) . "\n";

/** Create or fetch a page by slug (idempotent). */
function allied_seed_page( $title, $slug, $template = '', $excerpt = '', $content = '', $parent = 0 ) {
	$existing = get_page_by_path( $parent ? get_post_field( 'post_name', $parent ) . '/' . $slug : $slug );
	if ( $existing ) {
		return $existing->ID;
	}
	$id = wp_insert_post(
		array(
			'post_type'    => 'page',
			'post_status'  => 'publish',
			'post_title'   => $title,
			'post_name'    => $slug,
			'post_excerpt' => $excerpt,
			'post_content' => $content,
			'post_parent'  => $parent,
		)
	);
	if ( $template ) {
		update_post_meta( $id, '_wp_page_template', $template );
	}
	return $id;
}

$home = allied_seed_page( 'Home', 'home' );
$firm = allied_seed_page( 'The Firm', 'the-firm', 'page-templates/template-firm.php', 'A disciplined residential land developer across NE North Carolina and Hampton Roads, Virginia.', '<!-- wp:paragraph --><p>Allied Properties acquires, entitles, and develops residential land — delivering finished communities to national and regional homebuilders.</p><!-- /wp:paragraph -->' );
$cap  = allied_seed_page( 'Capabilities', 'capabilities', 'page-templates/template-capabilities.php', 'From acquisition and entitlement to horizontal development and finished lot delivery.' );
$part = allied_seed_page( 'Partners', 'partners', 'page-templates/template-partners.php', 'Builders, investors, and partners who help us bring communities to market.' );
$land = allied_seed_page( 'Sell Us Your Land', 'sell-us-your-land', 'page-templates/template-landowner.php', 'Have residential land in our footprint? We would like to hear from you.' );
$cont = allied_seed_page( 'Contact', 'contact', 'page-templates/template-contact.php' );
$port = allied_seed_page( 'Portals', 'portals', 'page-templates/template-portals.php', 'Secure areas for builder, investor, and partner relationships.' );
allied_seed_page( 'Builder', 'builder', 'page-templates/template-portal.php', '', '<!-- wp:paragraph --><p>Welcome to the Builder Portal. Lot delivery schedules and community specs appear here.</p><!-- /wp:paragraph -->', $port );
allied_seed_page( 'Investor', 'investor', 'page-templates/template-portal.php', '', '', $port );
allied_seed_page( 'Partner', 'partner', 'page-templates/template-portal.php', '', '', $port );
echo "Pages ready.\n";

update_option( 'show_on_front', 'page' );
update_option( 'page_on_front', $home );
update_option( 'blogname', 'Allied Properties' );
update_option( 'blogdescription', 'Residential land development across Northeastern North Carolina and Hampton Roads, Virginia.' );

// Primary menu (idempotent).
if ( ! wp_get_nav_menu_object( 'Primary Navigation' ) ) {
	$menu_id = wp_create_nav_menu( 'Primary Navigation' );
	foreach ( array( $firm => 'The Firm', $cap => 'Capabilities', $part => 'Partners', $land => 'Sell Us Your Land', $cont => 'Contact' ) as $pid => $t ) {
		wp_update_nav_menu_item( $menu_id, 0, array( 'menu-item-title' => $t, 'menu-item-object' => 'page', 'menu-item-object-id' => $pid, 'menu-item-type' => 'post_type', 'menu-item-status' => 'publish' ) );
	}
	$locations            = get_theme_mod( 'nav_menu_locations' );
	$locations['primary'] = $menu_id;
	set_theme_mod( 'nav_menu_locations', $locations );
	echo "Primary menu created.\n";
}

/** Create or fetch a community (idempotent). */
function allied_seed_community( $slug, $title, $content, $excerpt, $meta, $region, $status ) {
	if ( get_page_by_path( $slug, OBJECT, 'community' ) ) {
		return;
	}
	$id = wp_insert_post( array( 'post_type' => 'community', 'post_status' => 'publish', 'post_title' => $title, 'post_name' => $slug, 'post_content' => $content, 'post_excerpt' => $excerpt ) );
	foreach ( $meta as $k => $v ) {
		update_post_meta( $id, $k, $v );
	}
	wp_set_object_terms( $id, $region, 'region' );
	wp_set_object_terms( $id, $status, 'project_status' );
}

allied_seed_community(
	'lakeside-reserve', 'Lakeside Reserve',
	'<!-- wp:paragraph --><p>A 180-acre master-planned residential community delivering builder-ready lots in Chesapeake, Virginia.</p><!-- /wp:paragraph -->',
	'180-acre master-planned community in Chesapeake, VA.',
	array( '_allied_location' => 'Chesapeake, VA', '_allied_acreage' => '180 acres', '_allied_lots' => '420', '_allied_builder' => 'National Homebuilder', '_allied_year' => '2026' ),
	'Hampton Roads', 'In Development'
);
allied_seed_community(
	'albemarle-landing', 'Albemarle Landing',
	'<!-- wp:paragraph --><p>Finished single-family community delivered in Elizabeth City, North Carolina.</p><!-- /wp:paragraph -->',
	'Single-family community in Elizabeth City, NC.',
	array( '_allied_location' => 'Elizabeth City, NC', '_allied_acreage' => '95 acres', '_allied_lots' => '210', '_allied_year' => '2025' ),
	'Northeastern NC', 'Delivered'
);
echo "Communities ready.\n";

// Portal user (idempotent).
if ( ! username_exists( 'builder1' ) ) {
	$uid = wp_create_user( 'builder1', 'Builder!Preview123', 'builder1@example.com' );
	( new WP_User( $uid ) )->set_role( 'allied_builder' );
	echo "Portal user 'builder1' created (role: Builder Member).\n";
}

// Pretty permalinks + flush.
global $wp_rewrite;
update_option( 'permalink_structure', '/%postname%/' );
$wp_rewrite->set_permalink_structure( '/%postname%/' );
$wp_rewrite->flush_rules( true );
echo "Permalinks set & rewrite rules flushed.\n";
echo "SEED COMPLETE\n";
