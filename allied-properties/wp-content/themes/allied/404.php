<?php
/**
 * 404.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();
?>
<section class="section" style="text-align:center;">
	<div class="container container--narrow">
		<p class="eyebrow"><?php esc_html_e( 'Error 404', 'allied' ); ?></p>
		<h1><?php esc_html_e( 'This page could not be found.', 'allied' ); ?></h1>
		<p class="lead" style="margin-inline:auto;"><?php esc_html_e( 'The page you were looking for may have moved. Return home or explore our communities.', 'allied' ); ?></p>
		<p style="margin-top:var(--space-lg);">
			<a class="btn btn--primary" href="<?php echo esc_url( home_url( '/' ) ); ?>"><?php esc_html_e( 'Back to home', 'allied' ); ?></a>
			<a class="btn btn--ghost" href="<?php echo esc_url( home_url( '/communities/' ) ); ?>"><?php esc_html_e( 'View communities', 'allied' ); ?></a>
		</p>
	</div>
</section>
<?php get_footer(); ?>
