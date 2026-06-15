<?php
/**
 * Gated portals — roles & access control.
 *
 * Phase 1 portals are simple, manually-maintained gated areas (documents,
 * lists, PDFs). This file defines three custom roles and a helper to gate a
 * page to a given audience. It is written to interoperate with a membership
 * plugin if one is installed, but works on its own with native WP roles.
 *
 * Recommended companion plugins (see HANDOFF.md):
 *   - Members or User Role Editor (role management UI)
 *   - A simple membership/restriction plugin if you prefer drag-and-drop gating
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

/**
 * Portal definitions. Slug => label. Each maps to a WP role + a page template.
 */
function allied_portals() {
	return array(
		'builder'  => __( 'Builder Portal', 'allied' ),
		'investor' => __( 'Investor Portal', 'allied' ),
		'partner'  => __( 'Partner Portal', 'allied' ),
	);
}

/**
 * Register custom roles on theme activation. Each portal role is a subscriber
 * (read-only on the front end) tagged with a capability we check for gating.
 */
function allied_register_portal_roles() {
	foreach ( allied_portals() as $slug => $label ) {
		$role = "allied_$slug";
		if ( ! get_role( $role ) ) {
			add_role(
				$role,
				$label . ' Member',
				array(
					'read'                       => true,
					"access_portal_$slug"        => true,
				)
			);
		} else {
			// Ensure the capability exists even if the role pre-dates this code.
			$r = get_role( $role );
			$r->add_cap( "access_portal_$slug" );
		}
	}
	// Administrators can see every portal.
	$admin = get_role( 'administrator' );
	if ( $admin ) {
		foreach ( array_keys( allied_portals() ) as $slug ) {
			$admin->add_cap( "access_portal_$slug" );
		}
	}
}
add_action( 'after_switch_theme', 'allied_register_portal_roles' );

/**
 * Can the current user access a given portal?
 *
 * @param string $slug builder|investor|partner
 * @return bool
 */
function allied_can_access_portal( $slug ) {
	if ( ! is_user_logged_in() ) {
		return false;
	}
	return current_user_can( "access_portal_$slug" ) || current_user_can( 'manage_options' );
}

/**
 * Render a branded login + access-denied gate for a portal template.
 * Returns true if access is granted (caller should render the portal body),
 * false if the gate was shown instead.
 *
 * @param string $slug
 * @param string $label
 * @return bool
 */
function allied_portal_gate( $slug, $label ) {
	if ( allied_can_access_portal( $slug ) ) {
		return true;
	}

	echo '<div class="portal-login">';
	if ( is_user_logged_in() ) {
		// Logged in but lacks this portal's role.
		printf(
			'<h3>%s</h3><p class="text-muted">%s</p><p><a class="btn btn--ghost" href="%s">%s</a></p>',
			esc_html( $label ),
			esc_html__( 'Your account does not currently have access to this portal. Please contact Allied Properties to request access.', 'allied' ),
			esc_url( home_url( '/contact/' ) ),
			esc_html__( 'Request access', 'allied' )
		);
	} else {
		printf( '<h3>%s</h3><p class="text-muted">%s</p>', esc_html( $label ), esc_html__( 'Please sign in to continue.', 'allied' ) );
		wp_login_form(
			array(
				'redirect'       => get_permalink(),
				'label_username' => __( 'Email or username', 'allied' ),
				'label_log_in'   => __( 'Sign in', 'allied' ),
			)
		);
		printf(
			'<p class="form-note" style="margin-top:1rem"><a href="%s">%s</a></p>',
			esc_url( wp_lostpassword_url( get_permalink() ) ),
			esc_html__( 'Forgot your password?', 'allied' )
		);
	}
	echo '</div>';

	return false;
}
