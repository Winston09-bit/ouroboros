<?php
/**
 * Front page (Home).
 *
 * Built as the institutional homepage scaffold. Structure and design tokens
 * are in place; final palette/type and copy/photography are applied to match
 * the approved mockup (edit assets/css/tokens.css + the content below).
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();
?>

<!-- HERO -->
<section class="hero">
	<div class="hero__media">
		<?php
		if ( has_post_thumbnail() ) {
			the_post_thumbnail( 'allied-hero', array( 'fetchpriority' => 'high' ) );
		} else {
			echo '<img src="' . esc_url( get_template_directory_uri() . '/assets/img/hero-placeholder.svg' ) . '" alt="" />';
		}
		?>
	</div>
	<div class="container hero__inner">
		<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Residential Land Development', 'allied' ); ?></p>
		<h1 class="hero__title"><?php esc_html_e( 'We deliver finished communities builders can build on.', 'allied' ); ?></h1>
		<p class="hero__lead"><?php esc_html_e( 'Allied Properties acquires, entitles, and develops residential land across Northeastern North Carolina and Hampton Roads, Virginia — delivering shovel-ready lots to national and regional homebuilders.', 'allied' ); ?></p>
		<div class="hero__actions">
			<a class="btn btn--accent btn--lg" href="<?php echo esc_url( home_url( '/communities/' ) ); ?>"><?php esc_html_e( 'View communities', 'allied' ); ?></a>
			<a class="btn btn--on-dark btn--lg" href="<?php echo esc_url( home_url( '/the-firm/' ) ); ?>"><?php esc_html_e( 'About the firm', 'allied' ); ?></a>
		</div>
	</div>
</section>

<!-- POSITIONING -->
<section class="section">
	<div class="container split">
		<div>
			<p class="eyebrow"><?php esc_html_e( 'The Firm', 'allied' ); ?></p>
			<h2><?php esc_html_e( 'A disciplined land partner for serious capital.', 'allied' ); ?></h2>
		</div>
		<div>
			<p class="lead"><?php esc_html_e( 'We operate where we know the ground. From acquisition and entitlement through horizontal development, we manage risk like institutional investors and execute like local operators — so builders, lenders, and capital partners can count on the outcome.', 'allied' ); ?></p>
			<a class="link-arrow" href="<?php echo esc_url( home_url( '/capabilities/' ) ); ?>"><?php esc_html_e( 'Our capabilities', 'allied' ); ?></a>
		</div>
	</div>
</section>

<!-- PROOF / STATS -->
<section class="section--tight section--dark">
	<div class="container">
		<div class="statbar">
			<div class="stat"><div class="stat__num"><?php esc_html_e( '2', 'allied' ); ?></div><div class="stat__label"><?php esc_html_e( 'Core markets — NE North Carolina & Hampton Roads', 'allied' ); ?></div></div>
			<div class="stat"><div class="stat__num"><?php esc_html_e( 'End-to-end', 'allied' ); ?></div><div class="stat__label"><?php esc_html_e( 'Acquisition, entitlement, and horizontal development handled in-house', 'allied' ); ?></div></div>
			<div class="stat"><div class="stat__num"><?php esc_html_e( 'Builder-ready', 'allied' ); ?></div><div class="stat__label"><?php esc_html_e( 'Finished, shovel-ready lots delivered to builder specification', 'allied' ); ?></div></div>
			<div class="stat"><div class="stat__num"><?php esc_html_e( 'Institutional', 'allied' ); ?></div><div class="stat__label"><?php esc_html_e( 'Underwriting and reporting built for lenders and capital partners', 'allied' ); ?></div></div>
				<?php /* Once real figures are confirmed, swap a stat for e.g.:
				<div class="stat"><div class="stat__num">1,200+</div><div class="stat__label">Finished lots delivered</div></div> */ ?>
		</div>
	</div>
</section>

<!-- CAPABILITIES PREVIEW -->
<section class="section section--surface">
	<div class="container">
		<p class="eyebrow"><?php esc_html_e( 'What We Do', 'allied' ); ?></p>
		<h2><?php esc_html_e( 'Acquire. Entitle. Develop. Deliver.', 'allied' ); ?></h2>
		<div class="grid grid--4" style="margin-top:var(--space-lg);">
			<?php
			$caps = array(
				array( __( 'Acquisition', 'allied' ), __( 'Sourcing and underwriting residential land in markets we know intimately.', 'allied' ) ),
				array( __( 'Entitlement', 'allied' ), __( 'Rezoning, platting, and approvals managed with local relationships and rigor.', 'allied' ) ),
				array( __( 'Development', 'allied' ), __( 'Horizontal infrastructure delivered on schedule and to builder spec.', 'allied' ) ),
				array( __( 'Delivery', 'allied' ), __( 'Finished, shovel-ready communities handed to national and regional builders.', 'allied' ) ),
			);
			foreach ( $caps as $c ) {
				echo '<div class="feature"><h3>' . esc_html( $c[0] ) . '</h3><p class="text-muted">' . esc_html( $c[1] ) . '</p></div>';
			}
			?>
		</div>
	</div>
</section>

<!-- FEATURED COMMUNITIES -->
<section class="section">
	<div class="container">
		<div style="display:flex;justify-content:space-between;align-items:flex-end;gap:var(--space-md);flex-wrap:wrap;margin-bottom:var(--space-lg);">
			<div>
				<p class="eyebrow"><?php esc_html_e( 'Portfolio', 'allied' ); ?></p>
				<h2><?php esc_html_e( 'Selected communities', 'allied' ); ?></h2>
			</div>
			<a class="link-arrow" href="<?php echo esc_url( home_url( '/communities/' ) ); ?>"><?php esc_html_e( 'View all', 'allied' ); ?></a>
		</div>
		<div class="grid grid--3">
			<?php
			$featured = new WP_Query(
				array(
					'post_type'      => 'community',
					'posts_per_page' => 3,
					'no_found_rows'  => true,
				)
			);
			if ( $featured->have_posts() ) {
				while ( $featured->have_posts() ) {
					$featured->the_post();
					get_template_part( 'template-parts/card-community' );
				}
				wp_reset_postdata();
			} else {
				echo '<div class="empty-state" style="grid-column:1/-1;"><h3>' . esc_html__( 'Portfolio in preparation', 'allied' ) . '</h3><p>' . esc_html__( 'We are finalising our current community listings. Reach out to discuss active and upcoming projects.', 'allied' ) . '</p>';
					if ( current_user_can( 'edit_posts' ) ) { echo '<p class="form-note" style="margin-top:var(--space-sm);">' . esc_html__( 'Admin: add entries under Communities to populate this grid automatically.', 'allied' ) . '</p>'; }
					echo '</div>';
			}
			?>
		</div>
	</div>
</section>

<!-- PARTNERS -->
<section class="section section--surface">
	<div class="container">
		<p class="eyebrow" style="text-align:center;"><?php esc_html_e( 'Trusted By', 'allied' ); ?></p>
		<h2 style="text-align:center;margin-bottom:var(--space-lg);"><?php esc_html_e( 'Builder & capital partners', 'allied' ); ?></h2>
		<div class="logo-wall">
			<?php for ( $i = 0; $i < 10; $i++ ) : ?>
				<div class="logo-wall__cell"><span class="text-muted" style="font-size:var(--fs-small);"><?php esc_html_e( 'Logo', 'allied' ); ?></span></div>
			<?php endfor; ?>
		</div>
	</div>
</section>

<?php get_template_part( 'template-parts/cta-band' ); ?>

<?php get_footer(); ?>
