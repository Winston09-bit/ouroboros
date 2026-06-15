<?php
/**
 * Template Name: Partners
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
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Partners', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<?php if ( has_excerpt() ) : ?><p class="lead"><?php echo esc_html( get_the_excerpt() ); ?></p><?php endif; ?>
		</div>
	</section>

	<section class="section">
		<div class="container container--narrow entry-content stack"><?php the_content(); ?></div>
	</section>

	<!-- AUDIENCES -->
	<section class="section section--surface">
		<div class="container">
			<div class="grid grid--3">
				<?php
				$aud = array(
					array( __( 'Homebuilders', 'allied' ), __( 'National and regional builders seeking finished, builder-ready lots in growing Southeastern Virginia and Northeastern North Carolina markets.', 'allied' ), __( 'Builder Portal', 'allied' ), '/portals/builder/' ),
					array( __( 'Investors', 'allied' ), __( 'Capital partners and lenders looking for disciplined, well-underwritten residential land development opportunities.', 'allied' ), __( 'Investor Portal', 'allied' ), '/portals/investor/' ),
					array( __( 'Partners', 'allied' ), __( 'Brokers, engineers, municipalities, and landowners who help us bring quality communities to market.', 'allied' ), __( 'Partner Portal', 'allied' ), '/portals/partner/' ),
				);
				foreach ( $aud as $a ) {
					echo '<div class="portal-card">';
					echo '<h3>' . esc_html( $a[0] ) . '</h3>';
					echo '<p class="text-muted">' . esc_html( $a[1] ) . '</p>';
					echo '<a class="link-arrow" href="' . esc_url( home_url( $a[3] ) ) . '">' . esc_html( $a[2] ) . '</a>';
					echo '</div>';
				}
				?>
			</div>
		</div>
	</section>

	<!-- LOGO WALL -->
	<section class="section">
		<div class="container">
			<p class="eyebrow" style="text-align:center;"><?php esc_html_e( 'Selected Relationships', 'allied' ); ?></p>
			<h2 style="text-align:center;margin-bottom:var(--space-lg);"><?php esc_html_e( 'Builders & capital partners we work with', 'allied' ); ?></h2>
			<div class="logo-wall">
				<?php for ( $i = 0; $i < 15; $i++ ) : ?>
					<div class="logo-wall__cell"><span class="text-muted" style="font-size:var(--fs-small);"><?php esc_html_e( 'Logo', 'allied' ); ?></span></div>
				<?php endfor; ?>
			</div>
		</div>
	</section>
	<?php
endwhile;

get_template_part( 'template-parts/cta-band' );
get_footer();
