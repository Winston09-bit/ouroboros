<?php
/**
 * Template helpers & small front-end utilities.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }

/**
 * Output a responsive image for a post thumbnail, or a tasteful placeholder
 * block if no image has been set yet (so layouts never break during content
 * entry / before client photography is supplied).
 */
function allied_thumbnail( $size = 'allied-card', $class = '' ) {
	if ( has_post_thumbnail() ) {
		the_post_thumbnail( $size, array( 'class' => $class, 'loading' => 'lazy' ) );
	} else {
		printf(
			'<div class="%s" style="aspect-ratio:4/3;background:linear-gradient(135deg,var(--color-surface),var(--color-surface-2));display:grid;place-items:center;color:var(--color-muted);font-size:var(--fs-small);">%s</div>',
			esc_attr( $class ),
			esc_html__( 'Image to be supplied', 'allied' )
		);
	}
}

/**
 * Echo a community project meta value.
 */
function allied_project_meta( $key, $post_id = null ) {
	$post_id = $post_id ?: get_the_ID();
	return get_post_meta( $post_id, $key, true );
}

/**
 * Build the space-separated category list used by the JS portfolio filter.
 */
function allied_community_filter_terms( $post_id ) {
	$slugs = array();
	foreach ( array( 'region', 'project_status' ) as $tax ) {
		$terms = get_the_terms( $post_id, $tax );
		if ( $terms && ! is_wp_error( $terms ) ) {
			foreach ( $terms as $t ) {
				$slugs[] = $t->slug;
			}
		}
	}
	return implode( ' ', $slugs );
}

/**
 * Fallback menu when no menu has been assigned in Appearance > Menus.
 */
function allied_default_menu() {
	$items = array(
		'/the-firm/'     => __( 'The Firm', 'allied' ),
		'/capabilities/' => __( 'Capabilities', 'allied' ),
		'/communities/'  => __( 'Communities', 'allied' ),
		'/partners/'     => __( 'Partners', 'allied' ),
		'/landowners/'   => __( 'Landowners', 'allied' ),
		'/contact/'      => __( 'Contact', 'allied' ),
	);
	echo '<ul>';
	foreach ( $items as $path => $label ) {
		printf( '<li><a href="%s">%s</a></li>', esc_url( home_url( $path ) ), esc_html( $label ) );
	}
	echo '</ul>';
}

/**
 * Basic, dependency-free on-page SEO: meta description from excerpt/tagline
 * and Open Graph tags. (For richer SEO install Yoast/Rank Math — this avoids
 * a hard dependency while still giving good defaults.)
 */
function allied_seo_meta() {
	// Site-wide fallback so every page always has a meaningful description.
	$fallback = get_bloginfo( 'description' );
	if ( ! $fallback ) {
		$fallback = __( 'Allied Properties acquires, entitles, and develops residential land across Northeastern North Carolina and Hampton Roads, Virginia — delivering finished communities to national and regional homebuilders.', 'allied' );
	}

	$desc = '';
	if ( is_singular() ) {
		if ( has_excerpt() ) {
			$desc = get_the_excerpt();
		} else {
			$content = wp_strip_all_tags( get_the_content() );
			$desc    = $content ? $content : $fallback;
		}
	} elseif ( is_post_type_archive() || is_category() || is_tax() || is_tag() ) {
		$desc = wp_strip_all_tags( get_the_archive_description() );
		if ( ! $desc ) {
			$desc = $fallback;
		}
	} else {
		$desc = $fallback;
	}

	$desc = wp_trim_words( $desc, 30, '' );
	if ( $desc ) {
		printf( '<meta name="description" content="%s" />' . "\n", esc_attr( $desc ) );
		printf( '<meta property="og:description" content="%s" />' . "\n", esc_attr( $desc ) );
	}
	printf( '<meta property="og:title" content="%s" />' . "\n", esc_attr( wp_get_document_title() ) );
	printf( '<meta property="og:type" content="%s" />' . "\n", is_singular() ? 'article' : 'website' );
	printf( '<meta property="og:site_name" content="%s" />' . "\n", esc_attr( get_bloginfo( 'name' ) ) );
	if ( is_singular() && has_post_thumbnail() ) {
		printf( '<meta property="og:image" content="%s" />' . "\n", esc_url( get_the_post_thumbnail_url( null, 'allied-hero' ) ) );
	}
	echo '<meta name="twitter:card" content="summary_large_image" />' . "\n";
}
add_action( 'wp_head', 'allied_seo_meta', 5 );

/**
 * Trim default excerpt + custom "read more".
 */
add_filter( 'excerpt_more', function () { return '…'; } );
add_filter( 'excerpt_length', function () { return 28; } );

/**
 * Cleaner archive titles — drop WordPress's "Archives:", "Category:" prefixes.
 */
add_filter(
	'get_the_archive_title',
	function ( $title ) {
		if ( is_post_type_archive( 'community' ) ) {
			return __( 'Communities', 'allied' );
		}
		if ( is_tax() || is_category() || is_tag() ) {
			return single_term_title( '', false );
		}
		return $title;
	}
);
