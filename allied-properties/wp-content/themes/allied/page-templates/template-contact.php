<?php
/**
 * Template Name: Contact
 *
 * Drop a Contact Form 7 / WPForms shortcode into the page content for the
 * contact form. Firm contact details render alongside it.
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
			<p class="eyebrow" style="color:var(--color-accent);"><?php esc_html_e( 'Contact', 'allied' ); ?></p>
			<h1><?php the_title(); ?></h1>
			<?php if ( has_excerpt() ) : ?><p class="lead"><?php echo esc_html( get_the_excerpt() ); ?></p><?php endif; ?>
		</div>
	</section>

	<section class="section">
		<div class="container split" style="align-items:start;">
			<div class="stack">
				<h2><?php esc_html_e( 'Talk to the firm', 'allied' ); ?></h2>
				<p class="text-muted"><?php esc_html_e( 'For acquisitions, builder and investor inquiries, or partnership opportunities, reach us directly.', 'allied' ); ?></p>

				<div style="margin-top:var(--space-md);">
					<p class="eyebrow"><?php esc_html_e( 'Email', 'allied' ); ?></p>
					<p><a href="mailto:info@alliedproperties.example">info@alliedproperties.example</a></p>
				</div>
				<div>
					<p class="eyebrow"><?php esc_html_e( 'Markets', 'allied' ); ?></p>
					<p><?php esc_html_e( 'Northeastern North Carolina · Hampton Roads, Virginia', 'allied' ); ?></p>
				</div>
				<div>
					<p class="eyebrow"><?php esc_html_e( 'Landowners', 'allied' ); ?></p>
					<p><a class="link-arrow" href="<?php echo esc_url( home_url( '/sell-us-your-land/' ) ); ?>"><?php esc_html_e( 'Sell us your land', 'allied' ); ?></a></p>
				</div>
			</div>

			<div class="entry-content">
				<?php
				if ( get_the_content() ) {
					the_content();
				} else {
					echo '<p class="notice notice--info">' . esc_html__( 'Editor: add your Contact Form 7 or WPForms shortcode to this page to display the contact form here.', 'allied' ) . '</p>';
				}
				?>
			</div>
		</div>
	</section>
	<?php
endwhile;

get_footer();
