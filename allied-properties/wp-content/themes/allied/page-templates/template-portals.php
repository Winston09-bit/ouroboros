<?php
/**
 * Template Name: Portals Hub
 *
 * Overview page linking to the three gated portals. Assign this template to a
 * page at /portals/.
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
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Partner Portals', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<p class="lead"><?php echo esc_html( get_the_excerpt() ?: __( 'Secure, access-controlled areas for our builder, investor, and partner relationships.', 'allied' ) ); ?></p>
		</div>
	</section>

	<section class="section">
		<div class="container">
			<?php if ( get_the_content() ) : ?>
				<div class="container--narrow entry-content stack" style="margin:0 auto var(--space-xl);"><?php the_content(); ?></div>
			<?php endif; ?>

			<div class="portal-grid">
				<?php
				$portals = array(
					'builder'  => array( __( 'Builder Portal', 'allied' ), __( 'Lot delivery schedules, community specs, plats, and documents for our homebuilder partners.', 'allied' ) ),
					'investor' => array( __( 'Investor Portal', 'allied' ), __( 'Project updates, reporting, and capital documents for investors and lenders.', 'allied' ) ),
					'partner'  => array( __( 'Partner Portal', 'allied' ), __( 'Shared documents and resources for brokers, engineers, and municipal partners.', 'allied' ) ),
				);
				foreach ( $portals as $slug => $p ) {
					$can = allied_can_access_portal( $slug );
					echo '<div class="portal-card">';
					echo '<h3>' . esc_html( $p[0] ) . '</h3>';
					echo '<p class="text-muted">' . esc_html( $p[1] ) . '</p>';
					echo '<p class="portal-card__lock">' . ( $can ? '✓ ' . esc_html__( 'Access granted', 'allied' ) : '🔒 ' . esc_html__( 'Sign in required', 'allied' ) ) . '</p>';
					echo '<a class="btn ' . ( $can ? 'btn--primary' : 'btn--ghost' ) . '" href="' . esc_url( home_url( "/portals/$slug/" ) ) . '">' . ( $can ? esc_html__( 'Enter portal', 'allied' ) : esc_html__( 'Sign in', 'allied' ) ) . '</a>';
					echo '</div>';
				}
				?>
			</div>

			<p class="form-note" style="margin-top:var(--space-lg);text-align:center;">
				<?php esc_html_e( 'Need access? ', 'allied' ); ?><a href="<?php echo esc_url( home_url( '/contact/' ) ); ?>"><?php esc_html_e( 'Contact us to request portal credentials.', 'allied' ); ?></a>
			</p>
		</div>
	</section>
	<?php
endwhile;

get_footer();
