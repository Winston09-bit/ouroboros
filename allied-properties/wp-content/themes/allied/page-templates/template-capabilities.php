<?php
/**
 * Template Name: Capabilities
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
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Capabilities', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<?php if ( has_excerpt() ) : ?><p class="lead"><?php echo esc_html( get_the_excerpt() ); ?></p><?php endif; ?>
		</div>
	</section>

	<?php if ( get_the_content() ) : ?>
	<section class="section">
		<div class="container container--narrow entry-content stack"><?php the_content(); ?></div>
	</section>
	<?php endif; ?>

	<!-- CAPABILITY DETAIL -->
	<section class="section section--surface">
		<div class="container stack">
			<?php
			$capabilities = array(
				array(
					__( 'Land Acquisition', 'allied' ),
					__( 'We source and underwrite residential land across our footprint, structuring acquisitions that work for builders and capital partners alike. Off-market sourcing, disciplined due diligence, and clean closings.', 'allied' ),
				),
				array(
					__( 'Entitlement & Approvals', 'allied' ),
					__( 'Rezoning, preliminary and final platting, utility coordination, and municipal approvals — navigated with the local relationships and regulatory rigor that de-risk a deal.', 'allied' ),
				),
				array(
					__( 'Horizontal Development', 'allied' ),
					__( 'Grading, stormwater, roads, and wet/dry utilities delivered to builder specifications, managed on schedule and on budget.', 'allied' ),
				),
				array(
					__( 'Finished Lot Delivery', 'allied' ),
					__( 'Shovel-ready, builder-ready communities delivered to national and regional homebuilders — with the documentation lenders expect.', 'allied' ),
				),
			);
			$n = 0;
			foreach ( $capabilities as $cap ) {
				$n++;
				echo '<div class="split" style="padding-block:var(--space-lg);border-bottom:1px solid var(--color-line);">';
				echo '<div><span class="display" style="color:var(--color-line);font-family:var(--font-display);">' . sprintf( '%02d', $n ) . '</span><h2 style="margin-top:var(--space-sm);">' . esc_html( $cap[0] ) . '</h2></div>';
				echo '<div><p class="lead" style="color:var(--color-ink);">' . esc_html( $cap[1] ) . '</p></div>';
				echo '</div>';
			}
			?>
		</div>
	</section>
	<?php
endwhile;

get_template_part( 'template-parts/cta-band' );
get_footer();
