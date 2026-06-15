<?php
/**
 * Footer.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
?>
</main><!-- #main -->

<footer class="site-footer">
	<div class="container">
		<div class="site-footer__grid">
			<div>
				<a class="site-brand" href="<?php echo esc_url( home_url( '/' ) ); ?>" style="color:var(--color-on-dark);margin-bottom:1rem;">
					<span><strong><?php bloginfo( 'name' ); ?></strong></span>
				</a>
				<p style="max-width:34ch;"><?php echo esc_html( get_bloginfo( 'description' ) ?: __( 'Residential land development across Northeastern North Carolina and Hampton Roads, Virginia.', 'allied' ) ); ?></p>
			</div>

			<div>
				<h4><?php esc_html_e( 'Company', 'allied' ); ?></h4>
				<?php
				if ( has_nav_menu( 'footer' ) ) {
					wp_nav_menu( array( 'theme_location' => 'footer', 'container' => false, 'depth' => 1 ) );
				} else {
					echo '<ul>';
					printf( '<li><a href="%s">%s</a></li>', esc_url( home_url( '/the-firm/' ) ), esc_html__( 'The Firm', 'allied' ) );
					printf( '<li><a href="%s">%s</a></li>', esc_url( home_url( '/capabilities/' ) ), esc_html__( 'Capabilities', 'allied' ) );
					printf( '<li><a href="%s">%s</a></li>', esc_url( home_url( '/communities/' ) ), esc_html__( 'Communities', 'allied' ) );
					printf( '<li><a href="%s">%s</a></li>', esc_url( home_url( '/partners/' ) ), esc_html__( 'Partners', 'allied' ) );
					echo '</ul>';
				}
				?>
			</div>

			<div>
				<h4><?php esc_html_e( 'Engage', 'allied' ); ?></h4>
				<ul>
					<li><a href="<?php echo esc_url( home_url( '/sell-us-your-land/' ) ); ?>"><?php esc_html_e( 'Sell Us Your Land', 'allied' ); ?></a></li>
					<li><a href="<?php echo esc_url( home_url( '/portals/' ) ); ?>"><?php esc_html_e( 'Partner Portals', 'allied' ); ?></a></li>
					<li><a href="<?php echo esc_url( home_url( '/contact/' ) ); ?>"><?php esc_html_e( 'Contact', 'allied' ); ?></a></li>
				</ul>
			</div>

			<div>
				<h4><?php esc_html_e( 'Contact', 'allied' ); ?></h4>
				<ul>
					<li><a href="mailto:info@alliedproperties.example">info@alliedproperties.example</a></li>
					<li><?php esc_html_e( 'Northeastern NC · Hampton Roads, VA', 'allied' ); ?></li>
				</ul>
			</div>
		</div>

		<div class="site-footer__bottom">
			<span>&copy; <?php echo esc_html( gmdate( 'Y' ) ); ?> <?php bloginfo( 'name' ); ?>. <?php esc_html_e( 'All rights reserved.', 'allied' ); ?></span>
			<?php
			if ( has_nav_menu( 'legal' ) ) {
				wp_nav_menu( array( 'theme_location' => 'legal', 'container' => false, 'depth' => 1, 'menu_class' => 'legal-menu', 'items_wrap' => '<span>%3$s</span>' ) );
			}
			?>
		</div>
	</div>
</footer>

<?php wp_footer(); ?>
</body>
</html>
