<?php
/**
 * Reusable closing CTA band.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
?>
<section class="section">
	<div class="container">
		<div class="cta-band">
			<h2><?php esc_html_e( 'Have land in our footprint, or capital to deploy?', 'allied' ); ?></h2>
			<div class="hero__actions" style="margin:0;">
				<a class="btn btn--accent btn--lg" href="<?php echo esc_url( home_url( '/sell-us-your-land/' ) ); ?>"><?php esc_html_e( 'Sell us your land', 'allied' ); ?></a>
				<a class="btn btn--on-dark btn--lg" href="<?php echo esc_url( home_url( '/contact/' ) ); ?>"><?php esc_html_e( 'Talk to the firm', 'allied' ); ?></a>
			</div>
		</div>
	</div>
</section>
