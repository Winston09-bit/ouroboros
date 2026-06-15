<?php
/**
 * Template Name: The Firm / About + Leadership
 *
 * Intro + values pulled from the page content; leadership grid is editable
 * via the page editor (add team members below) — kept dependency-free.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();

while ( have_posts() ) :
	the_post();
	?>
	<section class="page-banner">
		<div class="container">
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'The Firm', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<?php if ( has_excerpt() ) : ?><p class="lead"><?php echo esc_html( get_the_excerpt() ); ?></p><?php endif; ?>
		</div>
	</section>

	<section class="section">
		<div class="container container--narrow entry-content stack">
			<?php the_content(); ?>
		</div>
	</section>

	<!-- VALUES -->
	<section class="section section--surface">
		<div class="container">
			<p class="eyebrow"><?php esc_html_e( 'How We Operate', 'allied' ); ?></p>
			<h2><?php esc_html_e( 'Principles that capital partners can underwrite.', 'allied' ); ?></h2>
			<div class="grid grid--3" style="margin-top:var(--space-lg);">
				<?php
				$values = array(
					array( __( 'Local knowledge', 'allied' ), __( 'We develop where we operate — deep market relationships in NE North Carolina and Hampton Roads.', 'allied' ) ),
					array( __( 'Institutional discipline', 'allied' ), __( 'Underwriting, reporting, and risk management built for lenders and capital partners.', 'allied' ) ),
					array( __( 'Builder-ready delivery', 'allied' ), __( 'Finished communities delivered to spec, on schedule, for national and regional builders.', 'allied' ) ),
				);
				foreach ( $values as $v ) {
					echo '<div class="feature"><h3>' . esc_html( $v[0] ) . '</h3><p class="text-muted">' . esc_html( $v[1] ) . '</p></div>';
				}
				?>
			</div>
		</div>
	</section>

	<!-- LEADERSHIP -->
	<section class="section">
		<div class="container">
			<p class="eyebrow"><?php esc_html_e( 'Leadership', 'allied' ); ?></p>
			<h2><?php esc_html_e( 'The people behind the firm', 'allied' ); ?></h2>
			<div class="grid grid--3" style="margin-top:var(--space-lg);">
				<?php for ( $i = 1; $i <= 3; $i++ ) : ?>
					<div class="person">
						<div class="person__photo" style="display:grid;place-items:center;color:var(--color-muted);font-size:var(--fs-small);"><?php esc_html_e( 'Photo', 'allied' ); ?></div>
						<h3 class="person__name"><?php esc_html_e( 'Name Surname', 'allied' ); ?></h3>
						<p class="person__role"><?php esc_html_e( 'Title', 'allied' ); ?></p>
						<p class="text-muted"><?php esc_html_e( 'Short professional biography. Replace this leadership block with real team members (editable in the page or, later, via a Team custom post type / ACF).', 'allied' ); ?></p>
					</div>
				<?php endfor; ?>
			</div>
			<p class="form-note" style="margin-top:var(--space-md);"><?php esc_html_e( 'Editor note: leadership cards are placeholders. They can be made fully editable with an ACF repeater or a Team CPT in a follow-up — flagged in HANDOFF.md.', 'allied' ); ?></p>
		</div>
	</section>
	<?php
endwhile;

get_template_part( 'template-parts/cta-band' );
get_footer();
