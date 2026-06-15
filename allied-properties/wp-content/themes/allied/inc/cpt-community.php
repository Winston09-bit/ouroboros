<?php
/**
 * Custom post type: Community / Project, plus a taxonomy for region/status,
 * and lightweight project meta fields (no plugin required).
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

/**
 * Register the "community" post type.
 */
function allied_register_community_cpt() {
	$labels = array(
		'name'               => __( 'Communities', 'allied' ),
		'singular_name'      => __( 'Community', 'allied' ),
		'add_new'            => __( 'Add Community', 'allied' ),
		'add_new_item'       => __( 'Add New Community', 'allied' ),
		'edit_item'          => __( 'Edit Community', 'allied' ),
		'new_item'           => __( 'New Community', 'allied' ),
		'view_item'          => __( 'View Community', 'allied' ),
		'search_items'       => __( 'Search Communities', 'allied' ),
		'not_found'          => __( 'No communities found', 'allied' ),
		'menu_name'          => __( 'Communities', 'allied' ),
	);

	register_post_type(
		'community',
		array(
			'labels'        => $labels,
			'public'        => true,
			'has_archive'   => true,
			'menu_icon'     => 'dashicons-admin-multisite',
			'menu_position' => 5,
			'rewrite'       => array( 'slug' => 'communities' ),
			'supports'      => array( 'title', 'editor', 'thumbnail', 'excerpt', 'page-attributes' ),
			'show_in_rest'  => true, // block editor + content-editable for admins
		)
	);
}
add_action( 'init', 'allied_register_community_cpt' );

/**
 * Region + Status taxonomies for filtering the portfolio grid.
 */
function allied_register_community_taxes() {
	register_taxonomy(
		'region',
		'community',
		array(
			'labels'       => array(
				'name'          => __( 'Regions', 'allied' ),
				'singular_name' => __( 'Region', 'allied' ),
			),
			'hierarchical' => true,
			'public'       => true,
			'show_in_rest' => true,
			'rewrite'      => array( 'slug' => 'region' ),
		)
	);
	register_taxonomy(
		'project_status',
		'community',
		array(
			'labels'       => array(
				'name'          => __( 'Statuses', 'allied' ),
				'singular_name' => __( 'Status', 'allied' ),
			),
			'hierarchical' => false,
			'public'       => true,
			'show_in_rest' => true,
			'rewrite'      => array( 'slug' => 'status' ),
		)
	);
}
add_action( 'init', 'allied_register_community_taxes' );

/**
 * Project meta box (lots, acreage, builder, location, year) without ACF.
 * If ACF is installed later, these can be migrated; kept dependency-free here.
 */
function allied_community_meta_box() {
	add_meta_box(
		'allied_community_details',
		__( 'Project Details', 'allied' ),
		'allied_community_meta_box_html',
		'community',
		'side',
		'default'
	);
}
add_action( 'add_meta_boxes', 'allied_community_meta_box' );

function allied_community_fields() {
	return array(
		'_allied_location' => __( 'Location (City, State)', 'allied' ),
		'_allied_acreage'  => __( 'Total Acreage', 'allied' ),
		'_allied_lots'     => __( 'Planned Lots', 'allied' ),
		'_allied_builder'  => __( 'Homebuilder Partner', 'allied' ),
		'_allied_year'     => __( 'Delivery / Year', 'allied' ),
	);
}

function allied_community_meta_box_html( $post ) {
	wp_nonce_field( 'allied_community_meta', 'allied_community_nonce' );
	foreach ( allied_community_fields() as $key => $label ) {
		$value = get_post_meta( $post->ID, $key, true );
		printf(
			'<p><label for="%1$s" style="font-weight:600;display:block;margin-bottom:4px;">%2$s</label>
			<input type="text" id="%1$s" name="%1$s" value="%3$s" style="width:100%%;" /></p>',
			esc_attr( $key ),
			esc_html( $label ),
			esc_attr( $value )
		);
	}
}

function allied_save_community_meta( $post_id ) {
	if ( ! isset( $_POST['allied_community_nonce'] ) || ! wp_verify_nonce( wp_unslash( $_POST['allied_community_nonce'] ), 'allied_community_meta' ) ) {
		return;
	}
	if ( defined( 'DOING_AUTOSAVE' ) && DOING_AUTOSAVE ) {
		return;
	}
	if ( ! current_user_can( 'edit_post', $post_id ) ) {
		return;
	}
	foreach ( array_keys( allied_community_fields() ) as $key ) {
		if ( isset( $_POST[ $key ] ) ) {
			update_post_meta( $post_id, $key, sanitize_text_field( wp_unslash( $_POST[ $key ] ) ) );
		}
	}
}
add_action( 'save_post_community', 'allied_save_community_meta' );
